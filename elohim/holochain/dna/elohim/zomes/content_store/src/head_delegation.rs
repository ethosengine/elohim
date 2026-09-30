//! Content-scoped grants and independently signed historical acceptance.
//! These are attributes of the existing canonical declaration, not new heads.
use super::*;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct HeadDelegationPayload {
    pub grantor: AgentPubKey,
    pub delegate: AgentPubKey,
    pub scope: String,
    pub valid_until: Timestamp,
    pub root_action_hash: ActionHash,
    pub dna_hash: DnaHash,
}

#[derive(Serialize, Deserialize, Debug, Clone, SerializedBytes)]
pub struct HeadDelegation {
    pub payload: HeadDelegationPayload,
    pub signature: Signature,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance: Option<HeadAcceptance>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HeadAcceptance {
    pub head_action_hash: ActionHash,
    pub accepted_at: Timestamp,
    pub signature: Signature,
}

/// The acceptance signature commits to the complete grant, including its
/// signature, and the exact authored action. A delegate cannot backdate this
/// root-author statement, nor reuse it for another version or network.
#[derive(Serialize, Debug)]
struct AcceptanceStatement<'a> {
    domain: &'static str,
    grant: &'a HeadDelegationPayload,
    grant_signature: &'a Signature,
    head_action_hash: &'a ActionHash,
    accepted_at: Timestamp,
}

fn statement<'a>(
    grant: &'a HeadDelegation,
    receipt: &'a HeadAcceptance,
) -> AcceptanceStatement<'a> {
    AcceptanceStatement {
        domain: "elohim:accepted-content-head:v1",
        grant: &grant.payload,
        grant_signature: &grant.signature,
        head_action_hash: &receipt.head_action_hash,
        accepted_at: receipt.accepted_at,
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GrantHeadDelegationInput {
    pub delegate: AgentPubKey,
    pub scope: String,
    pub valid_until: Timestamp,
    pub root_action_hash: ActionHash,
}

fn refused(reason: &str) -> WasmError {
    wasm_error!(WasmErrorInner::Guest(format!("head delegation: {reason}")))
}

#[hdk_extern]
pub fn grant_head_delegation(input: GrantHeadDelegationInput) -> ExternResult<HeadDelegation> {
    let grantor = agent_info()?.agent_initial_pubkey;
    if input.delegate == grantor {
        return Err(refused("delegate must differ from the grantor"));
    }
    if input.scope.is_empty() || input.scope.contains('*') {
        return Err(refused("scope must name one exact content identity"));
    }
    let root = canonical_identity_root(&input.scope, GetStrategy::Network)?
        .ok_or_else(|| refused("immutable root is not available"))?;
    if root.action_address() != &input.root_action_hash || root.action().author() != &grantor {
        return Err(refused(
            "only the independently resolved root author may grant this root",
        ));
    }
    if input.valid_until <= sys_time()? {
        return Err(refused("expired: valid_until must be in the future"));
    }
    let payload = HeadDelegationPayload {
        grantor: grantor.clone(),
        delegate: input.delegate,
        scope: input.scope,
        valid_until: input.valid_until,
        root_action_hash: input.root_action_hash,
        dna_hash: dna_info()?.hash,
    };
    let signature = sign(grantor, &payload)?;
    Ok(HeadDelegation {
        payload,
        signature,
        acceptance: None,
    })
}

/// Structural checks shared by admission and historical verification. Kept
/// separate from HDK calls so the security distinctions have small regressions.
fn check_subject(
    p: &HeadDelegationPayload,
    me: &AgentPubKey,
    author: &AgentPubKey,
    id: &str,
    root: &ActionHash,
    dna: &DnaHash,
) -> Result<(), &'static str> {
    if p.grantor == p.delegate {
        return Err("delegate must differ from the grantor");
    }
    if &p.grantor != author {
        return Err("grantor is not the immutable root author");
    }
    if &p.delegate != me {
        return Err("delegate is not the caller");
    }
    if p.scope != id || p.scope.contains('*') {
        return Err("scope does not cover this exact content identity");
    }
    if &p.root_action_hash != root {
        return Err("immutable root differs");
    }
    if &p.dna_hash != dna {
        return Err("DNA context differs");
    }
    Ok(())
}

const REVOCATION_TAG: &[u8] = b"head-delegation-revoked:v1:";
fn revocation_tag(grant: &HeadDelegation) -> Vec<u8> {
    [REVOCATION_TAG, grant.signature.as_ref()].concat()
}

fn is_revoked(
    grant: &HeadDelegation,
    strategy: GetStrategy,
    accepted_at: Option<Timestamp>,
) -> ExternResult<bool> {
    let query = LinkQuery::try_new(
        grant.payload.root_action_hash.clone(),
        LinkTypes::IdToContent,
    )?;
    // Deleted revocations still count. Existing integrity permits link deletion;
    // deletion by any agent cannot erase the root author's signed invalidation.
    let details: Vec<(SignedActionHashed, Vec<SignedActionHashed>)> =
        get_links_details(query, strategy)?.into();
    let tag = revocation_tag(grant);
    Ok(details.into_iter().any(|(create, _)| {
        create.action().author() == &grant.payload.grantor
            && accepted_at.is_none_or(|accepted| create.action().timestamp() <= accepted)
            && matches!(&create.action().data, ActionData::CreateLink(link)
                if link.tag.0 == tag && link.target_address == AnyLinkableHash::from(grant.payload.delegate.clone()))
    }))
}

fn verify_grant(
    grant: &HeadDelegation,
    me: &AgentPubKey,
    author: &AgentPubKey,
    id: &str,
    root: &ActionHash,
    _strategy: GetStrategy,
) -> ExternResult<()> {
    check_subject(&grant.payload, me, author, id, root, &dna_info()?.hash).map_err(refused)?;
    if !verify_signature(
        grant.payload.grantor.clone(),
        grant.signature.clone(),
        &grant.payload,
    )? {
        return Err(refused("signature does not verify against root author"));
    }
    Ok(())
}

pub(crate) fn verify_head_delegation(
    grant: &HeadDelegation,
    me: &AgentPubKey,
    author: &AgentPubKey,
    id: &str,
) -> ExternResult<()> {
    let root = canonical_identity_root(id, GetStrategy::Network)?
        .ok_or_else(|| refused("immutable root is not available"))?;
    verify_grant(
        grant,
        me,
        author,
        id,
        root.action_address(),
        GetStrategy::Network,
    )?;
    if is_revoked(grant, GetStrategy::Network, None)? {
        return Err(refused("revoked for new publication by root author"));
    }
    if grant.payload.valid_until <= sys_time()? {
        return Err(refused("expired for new publication"));
    }
    Ok(())
}

fn check_receipt(grant: &HeadDelegation, head: &ActionHash) -> Result<(), &'static str> {
    let receipt = grant
        .acceptance
        .as_ref()
        .ok_or("root-author acceptance receipt is required")?;
    if &receipt.head_action_hash != head {
        return Err("acceptance names a different version");
    }
    if receipt.accepted_at >= grant.payload.valid_until {
        return Err("acceptance occurred after expiry");
    }
    Ok(())
}

/// Historical verification deliberately has no current-time expiry check. Its
/// evidence is a ROOT AUTHOR signature over the exact accepted version, never
/// the delegate's assertion of an old timestamp.
pub(crate) fn verify_accepted_head(
    grant: &HeadDelegation,
    me: &AgentPubKey,
    author: &AgentPubKey,
    id: &str,
    root: &ActionHash,
    head: &ActionHash,
    strategy: GetStrategy,
) -> ExternResult<()> {
    verify_grant(grant, me, author, id, root, strategy)?;
    check_receipt(grant, head).map_err(refused)?;
    let receipt = grant
        .acceptance
        .as_ref()
        .ok_or_else(|| refused("acceptance missing"))?;
    if !verify_signature(
        author.clone(),
        receipt.signature.clone(),
        &statement(grant, receipt),
    )? {
        return Err(refused(
            "acceptance signature does not verify against root author",
        ));
    }
    // Revocation ends future exercise; a prior root-author acceptance remains
    // valid. The compared clocks are both root-author signed, never supplied
    // solely by the delegate. Withdrawal/replacement is a separate root-author
    // canonical declaration, ordered by the same election.
    if is_revoked(grant, strategy, Some(receipt.accepted_at))? {
        return Err(refused("acceptance did not precede root-author revocation"));
    }
    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AcceptDelegatedHeadInput {
    pub id: String,
    pub head_action_hash: ActionHash,
    pub delegation: HeadDelegation,
}

#[hdk_extern]
pub fn accept_delegated_head(mut input: AcceptDelegatedHeadInput) -> ExternResult<HeadDelegation> {
    let author = agent_info()?.agent_initial_pubkey;
    if author != input.delegation.payload.grantor {
        return Err(refused("only root author may accept"));
    }
    verify_head_delegation(
        &input.delegation,
        &input.delegation.payload.delegate,
        &author,
        &input.id,
    )?;
    let lineage = correction::get_content_lineage(correction::GetContentLineageInput {
        action_hash: input.head_action_hash.clone(),
        local: false,
    })?;
    if lineage.content_id != input.id
        || lineage.root_action_hash != input.delegation.payload.root_action_hash
    {
        return Err(refused(
            "accepted version does not descend from delegated root",
        ));
    }
    let version = get(input.head_action_hash.clone(), GetOptions::default())?
        .ok_or_else(|| refused("accepted version unavailable"))?;
    if version.action().author() != &author
        && version.action().author() != &input.delegation.payload.delegate
    {
        return Err(refused(
            "accepted version was not authored by grantor or delegate",
        ));
    }
    let mut receipt = HeadAcceptance {
        head_action_hash: input.head_action_hash,
        accepted_at: sys_time()?,
        signature: Signature([0; 64]),
    };
    if receipt.accepted_at >= input.delegation.payload.valid_until {
        return Err(refused("expired during acceptance"));
    }
    receipt.signature = sign(author, &statement(&input.delegation, &receipt))?;
    input.delegation.acceptance = Some(receipt);
    Ok(input.delegation)
}

/// Explicit revocation ends future exercise, independently of expiry. Prior
/// root-accepted publications remain historical evidence; replacing one is a
/// separate root-author declaration, never an implicit rollback.
/// No administrative credential or enrolled-device status grants this power.
#[hdk_extern]
pub fn revoke_head_delegation(grant: HeadDelegation) -> ExternResult<ActionHash> {
    let author = agent_info()?.agent_initial_pubkey;
    if author != grant.payload.grantor {
        return Err(refused("only root author may revoke"));
    }
    let root = canonical_identity_root(&grant.payload.scope, GetStrategy::Network)?
        .ok_or_else(|| refused("immutable root unavailable"))?;
    check_subject(
        &grant.payload,
        &grant.payload.delegate,
        root.action().author(),
        &grant.payload.scope,
        root.action_address(),
        &dna_info()?.hash,
    )
    .map_err(refused)?;
    if !verify_signature(author, grant.signature.clone(), &grant.payload)? {
        return Err(refused("invalid signature"));
    }
    create_link(
        grant.payload.root_action_hash.clone(),
        grant.payload.delegate.clone(),
        LinkTypes::IdToContent,
        revocation_tag(&grant),
    )
}

pub(crate) fn authorize_author_or_delegate(
    me: &AgentPubKey,
    root_author: &AgentPubKey,
    id: &str,
    delegation: Option<&HeadDelegation>,
    what: &str,
) -> ExternResult<Option<HeadDelegation>> {
    match delegation {
        Some(d) => { verify_head_delegation(d, me, root_author, id)?; Ok(Some(d.clone())) }
        None if me == root_author => Ok(None),
        None => Err(wasm_error!(WasmErrorInner::Guest(format!(
            "{what}: agent {me:?} is not the author of content '{id}' (author {root_author:?}) and carries no head delegation"
        )))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grant() -> HeadDelegation {
        HeadDelegation {
            payload: HeadDelegationPayload {
                grantor: AgentPubKey::from_raw_36(vec![1; 36]),
                delegate: AgentPubKey::from_raw_36(vec![2; 36]),
                scope: "fct-course".into(),
                valid_until: Timestamp::from_micros(100),
                root_action_hash: ActionHash::from_raw_36(vec![3; 36]),
                dna_hash: DnaHash::from_raw_36(vec![4; 36]),
            },
            signature: Signature([5; 64]),
            acceptance: Some(HeadAcceptance {
                head_action_hash: ActionHash::from_raw_36(vec![6; 36]),
                accepted_at: Timestamp::from_micros(99),
                signature: Signature([7; 64]),
            }),
        }
    }

    #[test]
    fn context_and_subject_are_exact_not_prefix_or_role_authority() {
        let g = grant();
        let p = &g.payload;
        let check = |p: &HeadDelegationPayload| {
            check_subject(
                p,
                &g.payload.delegate,
                &g.payload.grantor,
                "fct-course",
                &g.payload.root_action_hash,
                &g.payload.dna_hash,
            )
        };
        assert_eq!(check(p), Ok(()));
        let mut altered = p.clone();
        altered.scope = "fct-*".into();
        assert!(check(&altered).unwrap_err().contains("scope"));
        altered = p.clone();
        altered.root_action_hash = ActionHash::from_raw_36(vec![9; 36]);
        assert_eq!(check(&altered), Err("immutable root differs"));
        altered = p.clone();
        altered.dna_hash = DnaHash::from_raw_36(vec![9; 36]);
        assert_eq!(check(&altered), Err("DNA context differs"));
        altered = p.clone();
        altered.grantor = AgentPubKey::from_raw_36(vec![9; 36]);
        assert!(check(&altered).unwrap_err().contains("root author"));
        altered = p.clone();
        altered.delegate = AgentPubKey::from_raw_36(vec![9; 36]);
        assert!(check(&altered).unwrap_err().contains("caller"));
    }

    #[test]
    fn historical_receipt_is_bound_to_exact_head_and_prior_expiry() {
        let mut g = grant();
        let head = g.acceptance.as_ref().unwrap().head_action_hash.clone();
        // Current time is deliberately absent: this receipt remains valid after
        // expiry; the separate live admission check refuses any new declare.
        assert_eq!(check_receipt(&g, &head), Ok(()));
        assert!(check_receipt(&g, &ActionHash::from_raw_36(vec![9; 36])).is_err());
        g.acceptance.as_mut().unwrap().accepted_at = g.payload.valid_until;
        assert_eq!(
            check_receipt(&g, &head),
            Err("acceptance occurred after expiry")
        );
        g.acceptance = None;
        assert!(check_receipt(&g, &head).unwrap_err().contains("required"));
    }

    #[test]
    fn acceptance_signing_bytes_change_for_version_grant_and_time() {
        let g = grant();
        let bytes = |g: &HeadDelegation| {
            holochain_serialized_bytes::encode(&statement(g, g.acceptance.as_ref().unwrap()))
                .unwrap()
        };
        let original = bytes(&g);
        let mut other = g.clone();
        other.signature = Signature([8; 64]);
        assert_ne!(original, bytes(&other));
        other = g.clone();
        other.acceptance.as_mut().unwrap().head_action_hash = ActionHash::from_raw_36(vec![9; 36]);
        assert_ne!(original, bytes(&other));
        other = g.clone();
        other.acceptance.as_mut().unwrap().accepted_at = Timestamp::from_micros(98);
        assert_ne!(original, bytes(&other));
        other = g.clone();
        other.payload.dna_hash = DnaHash::from_raw_36(vec![9; 36]);
        assert_ne!(original, bytes(&other));
    }

    #[test]
    fn accepted_grant_fits_holochain_link_tag_and_round_trips() {
        let g = grant();
        let tag = canonical_tag_with_delegation(CANONICAL_TAG_EARNED, Some(&g));
        assert!(
            tag.len() < MAX_CARRIED_LINK_TAG_BYTES,
            "{} bytes",
            tag.len()
        );
        let decoded = delegation_from_tag(&tag).unwrap();
        assert_eq!(decoded.payload, g.payload);
        assert_eq!(
            decoded.acceptance.unwrap().head_action_hash,
            g.acceptance.unwrap().head_action_hash
        );
    }
    #[test]
    fn replayed_declaration_cannot_refresh_accepted_version_rank() {
        let old = grant();
        let make = |link_byte, time| CanonicalCandidate {
            is_earned: true,
            timestamp: Timestamp::from_micros(time),
            link_hash: ActionHash::from_raw_36(vec![link_byte; 36]),
            target: old.acceptance.as_ref().unwrap().head_action_hash.clone(),
            ordering_hash: None,
        };
        let mut replay = make(255, i64::MAX);
        use_accepted_ordering(&mut replay, &old);
        let mut next_grant = old.clone();
        next_grant.acceptance.as_mut().unwrap().accepted_at = Timestamp::from_micros(100);
        next_grant.acceptance.as_mut().unwrap().head_action_hash =
            ActionHash::from_raw_36(vec![8; 36]);
        let mut next = make(1, 101);
        next.target = next_grant
            .acceptance
            .as_ref()
            .unwrap()
            .head_action_hash
            .clone();
        use_accepted_ordering(&mut next, &next_grant);
        assert_eq!(
            select_canonical_winner(vec![replay.clone(), next.clone()])
                .unwrap()
                .target,
            next.target
        );
        // Equal acceptance clocks also have stable version order, independent
        // of arbitrary replacement CreateLink bytes or their arrival order.
        replay.timestamp = next.timestamp;
        let winner = select_canonical_winner(vec![next.clone(), replay.clone()]).unwrap();
        replay.link_hash = ActionHash::from_raw_36(vec![0; 36]);
        assert_eq!(
            winner.target,
            select_canonical_winner(vec![replay, next]).unwrap().target
        );
    }
}
