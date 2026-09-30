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
    /// Unique root-author source-chain action that issued this grant. Omitted
    /// only on legacy v2 receipts so their original signature bytes survive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issuance_action_hash: Option<ActionHash>,
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
    /// Root-author CreateLink witnessing this exact grant/version acceptance.
    pub witness_action_hash: ActionHash,
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
    witness_action_hash: &'a ActionHash,
    accepted_at: Timestamp,
}

fn statement<'a>(
    grant: &'a HeadDelegation,
    receipt: &'a HeadAcceptance,
) -> AcceptanceStatement<'a> {
    AcceptanceStatement {
        domain: if grant.payload.issuance_action_hash.is_some() {
            "elohim:accepted-content-head:v3"
        } else {
            "elohim:accepted-content-head:v2"
        },
        grant: &grant.payload,
        grant_signature: &grant.signature,
        head_action_hash: &receipt.head_action_hash,
        witness_action_hash: &receipt.witness_action_hash,
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
    // The root-author-signed CreateLink is the unique issuance anchor. Its
    // ActionHash is not known until after the source-chain write, so the grant
    // signature is made over that hash in the following step. The root and
    // delegate are in the action itself; the compact tag fixes the expiry.
    let issuance_action_hash = create_link(
        input.root_action_hash.clone(),
        input.delegate.clone(),
        LinkTypes::IdToContent,
        LinkTag::new(issuance_tag(input.valid_until)),
    )?;
    let payload = HeadDelegationPayload {
        grantor: grantor.clone(),
        delegate: input.delegate,
        scope: input.scope,
        valid_until: input.valid_until,
        root_action_hash: input.root_action_hash,
        dna_hash: dna_info()?.hash,
        issuance_action_hash: Some(issuance_action_hash),
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

const ISSUANCE_TAG: &[u8] = b"head-delegation-issued:v1:";
fn issuance_tag(valid_until: Timestamp) -> Vec<u8> {
    [ISSUANCE_TAG, &valid_until.as_micros().to_be_bytes()].concat()
}

const REVOCATION_TAG_V1: &[u8] = b"head-delegation-revoked:v1:";
const REVOCATION_TAG_V2: &[u8] = b"head-delegation-revoked:v2:";
fn revocation_tag(grant: &HeadDelegation) -> Vec<u8> {
    match &grant.payload.issuance_action_hash {
        Some(issuance) => [REVOCATION_TAG_V2, issuance.get_raw_39()].concat(),
        None => [REVOCATION_TAG_V1, grant.signature.as_ref()].concat(),
    }
}

fn acceptance_history_anchor(grant: &HeadDelegation) -> &ActionHash {
    grant
        .payload
        .issuance_action_hash
        .as_ref()
        .unwrap_or(&grant.payload.root_action_hash)
}

fn history_predecessor_matches(
    prior_sequence: u32,
    prior_timestamp: Timestamp,
    current_sequence: u32,
    current_timestamp: Timestamp,
) -> bool {
    prior_sequence.checked_add(1) == Some(current_sequence) && prior_timestamp < current_timestamp
}

fn issuance_action_matches(
    grant: &HeadDelegation,
    record: &Record,
    expected_type: ScopedLinkType,
) -> bool {
    let Some(expected_hash) = &grant.payload.issuance_action_hash else {
        return false;
    };
    if record.action_address() != expected_hash
        || record.action().author() != &grant.payload.grantor
        || !matches!(&record.action().data, ActionData::CreateLink(link)
            if link.base_address == AnyLinkableHash::from(grant.payload.root_action_hash.clone())
                && link.target_address == AnyLinkableHash::from(grant.payload.delegate.clone())
                && link.tag.0 == issuance_tag(grant.payload.valid_until))
    {
        return false;
    }
    matches!(&record.action().data, ActionData::CreateLink(link)
        if link.zome_index == expected_type.zome_index
            && link.link_type == expected_type.zome_type)
}

fn verify_issuance_record(
    grant: &HeadDelegation,
    root: &Record,
    record: &Record,
) -> ExternResult<()> {
    let expected_type: ScopedLinkType = LinkTypes::IdToContent.try_into()?;
    if !issuance_context_matches(grant, root, record, expected_type)
        || !verify_signature(
            grant.payload.grantor.clone(),
            record.signature().clone(),
            record.action(),
        )?
    {
        return Err(refused(
            "native issuance action does not follow the immutable root or match the signed grant",
        ));
    }
    Ok(())
}

/// Pure native-record boundary shared by issuance verification and regression
/// tests. Cryptographic authenticity is checked immediately after this
/// predicate by `verify_issuance_record` through Holochain's signature host.
fn issuance_context_matches(
    grant: &HeadDelegation,
    root: &Record,
    record: &Record,
    expected_type: ScopedLinkType,
) -> bool {
    issuance_action_matches(grant, record, expected_type)
        && root.action_address() == &grant.payload.root_action_hash
        && root.action().author() == &grant.payload.grantor
        && root.action().action_seq() < record.action().action_seq()
        && root.action().timestamp() < record.action().timestamp()
        && record.action().timestamp() < grant.payload.valid_until
}

fn is_revoked(
    grant: &HeadDelegation,
    strategy: GetStrategy,
    accepted_sequence: Option<u32>,
    budget: Option<&AcceptanceBudget>,
) -> ExternResult<bool> {
    let query = LinkQuery::try_new(
        grant.payload.root_action_hash.clone(),
        LinkTypes::IdToContent,
    )?
    .tag_prefix(LinkTag::new(revocation_tag(grant)));
    // Deleted revocations still count. Existing integrity permits link deletion;
    // deletion by any agent cannot erase the root author's signed invalidation.
    let details: Vec<(SignedActionHashed, Vec<SignedActionHashed>)> = match budget {
        Some(budget) => budget.links(query, strategy)?,
        None => get_links_details(query, strategy)?,
    }
    .into();
    let tag = revocation_tag(grant);
    Ok(details.into_iter().any(|(create, _)| {
        create.action().author() == &grant.payload.grantor
            && accepted_sequence.is_none_or(|accepted| create.action().action_seq() <= accepted)
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
    let budget = AcceptanceBudget::start()?;
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
    let issuance_hash = grant
        .payload
        .issuance_action_hash
        .as_ref()
        .ok_or_else(|| refused("legacy grant cannot authorize a new publication"))?;
    let issuance = budget
        .record(issuance_hash.clone(), GetStrategy::Network)?
        .ok_or_else(|| refused("grant issuance action not retrievable — PENDING"))?;
    verify_issuance_record(grant, &root, &issuance)?;
    if is_revoked(grant, GetStrategy::Network, None, Some(&budget))? {
        return Err(refused("revoked for new publication by root author"));
    }
    if grant.payload.valid_until <= sys_time()? {
        return Err(refused("expired for new publication"));
    }
    Ok(())
}

/// Reuse the native batch resolver's wall budget. A record cap alone cannot
/// bound sequential network waits. Check before every host read and pass only
/// the remaining allowance to network requests; one host/database operation can
/// still overshoot because HDK host calls are synchronous and not cancellable.
struct AcceptanceBudget(Timestamp);
impl AcceptanceBudget {
    fn start() -> ExternResult<Self> {
        Ok(Self(sys_time()?))
    }

    fn options(&self, strategy: GetStrategy) -> ExternResult<GetOptions> {
        acceptance_get_options(self.0, sys_time()?, strategy)
            .ok_or_else(|| refused("acceptance verification time budget exceeded — PENDING"))
    }

    fn record(&self, hash: ActionHash, strategy: GetStrategy) -> ExternResult<Option<Record>> {
        let result = get(hash, self.options(strategy)?);
        self.options(strategy)?;
        result.map_err(|error| {
            refused(&format!(
                "acceptance record lookup failed — PENDING: {error:?}"
            ))
        })
    }

    fn links(&self, query: LinkQuery, strategy: GetStrategy) -> ExternResult<LinkDetails> {
        let input = GetLinksInput::from_query(query, self.options(strategy)?);
        let result = HDK.with(|h| h.borrow().get_links_details(vec![input]));
        self.options(strategy)?;
        result
            .map_err(|error| {
                refused(&format!(
                    "acceptance link lookup failed — PENDING: {error:?}"
                ))
            })?
            .into_iter()
            .next()
            .ok_or_else(|| refused("acceptance link history not retrievable — PENDING"))
    }
}

fn acceptance_remaining_ms(start: Timestamp, now: Timestamp) -> Option<u32> {
    BATCH_BUDGET_DEFAULT_MS
        .checked_sub(elapsed_ms_since(start, now))
        .filter(|remaining| *remaining > 0)
}

fn acceptance_get_options(
    start: Timestamp,
    now: Timestamp,
    strategy: GetStrategy,
) -> Option<GetOptions> {
    acceptance_remaining_ms(start, now)
        .map(|remaining| GetOptions::from(strategy).with_timeout_ms(u64::from(remaining)))
}

const ACCEPTANCE_WITNESS_TAG: &[u8] = b"head-acceptance:v2:";
const MAX_ACCEPTANCE_WITNESSES: usize = 16;

fn acceptance_tag(grant: &HeadDelegation, head: &ActionHash) -> Vec<u8> {
    [
        ACCEPTANCE_WITNESS_TAG,
        grant.signature.as_ref(),
        head.get_raw_39(),
    ]
    .concat()
}

/// Link integrity is generic, so every consumer verifies the actual action,
/// not a supplied timestamp, author string, or a portable carried record.
fn verify_acceptance_witness(
    grant: &HeadDelegation,
    head: &ActionHash,
    witness: &Record,
) -> ExternResult<()> {
    let action = witness.action();
    if action.author() != &grant.payload.grantor
        || !verify_signature(action.author().clone(), witness.signature().clone(), action)?
    {
        return Err(refused(
            "acceptance witness is not signed by the root author",
        ));
    }
    let expected_type: ScopedLinkType = LinkTypes::IdToContent.try_into()?;
    if !matches!(&action.data, ActionData::CreateLink(link)
        if link.base_address == AnyLinkableHash::from(grant.payload.root_action_hash.clone())
            && link.target_address == AnyLinkableHash::from(head.clone())
            && link.zome_index == expected_type.zome_index
            && link.link_type == expected_type.zome_type
            && link.tag.0 == acceptance_tag(grant, head))
    {
        return Err(refused(
            "acceptance witness context does not match exact grant and head",
        ));
    }
    if action.timestamp() >= grant.payload.valid_until {
        return Err(refused("acceptance witness occurred after expiry"));
    }
    Ok(())
}

fn witnessed_acceptance(
    grant: &HeadDelegation,
    head: &ActionHash,
    strategy: GetStrategy,
) -> ExternResult<Option<Record>> {
    let budget = AcceptanceBudget::start()?;
    let tag = acceptance_tag(grant, head);
    let details: Vec<(SignedActionHashed, Vec<SignedActionHashed>)> = budget
        .links(
            LinkQuery::try_new(
                grant.payload.root_action_hash.clone(),
                LinkTypes::IdToContent,
            )?
            .tag_prefix(LinkTag::new(tag.clone())),
            strategy,
        )?
        .into();
    // Host get_links_details does not apply an author filter. Filter actual
    // signed authors before the candidate cap; never trust link-supplied metadata.
    // Deleted links still witness prior acceptance: deletion is not revocation.
    let mut candidates: Vec<_> = details
        .into_iter()
        .map(|(create, _)| create)
        .filter(|create| create.action().author() == &grant.payload.grantor)
        .filter(|create| {
            matches!(&create.action().data, ActionData::CreateLink(link)
            if link.tag.0 == tag && link.target_address == AnyLinkableHash::from(head.clone()))
        })
        .collect();
    candidates.sort_by_key(|create| (create.action().action_seq(), create.as_hash().clone()));
    candidates.dedup_by(|a, b| a.as_hash() == b.as_hash());
    if candidates.len() > MAX_ACCEPTANCE_WITNESSES {
        return Err(refused("acceptance witness candidate budget exceeded"));
    }
    if candidates
        .windows(2)
        .any(|pair| pair[0].action().action_seq() == pair[1].action().action_seq())
    {
        return Err(refused("acceptance witness source-chain conflict"));
    }
    let Some(first) = candidates.into_iter().next() else {
        return Ok(None);
    };
    let record = budget
        .record(first.as_hash().clone(), strategy)?
        .ok_or_else(|| refused("acceptance witness not retrievable — PENDING"))?;
    verify_acceptance_witness(grant, head, &record)?;
    Ok(Some(record))
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

// A local link index cannot prove that an earlier revocation is absent. For v3,
// walk from the actual acceptance witness to the unique signed issuance action;
// legacy v2 receipts retain their original root-bounded path. Missing history is
// PENDING, never inferred from timestamps or a link-index miss.
const MAX_ACCEPTANCE_AUTHOR_HISTORY: usize = 4096;
fn verify_acceptance_author_history(
    grant: &HeadDelegation,
    witness: &Record,
    strategy: GetStrategy,
    budget: &AcceptanceBudget,
) -> ExternResult<()> {
    let mut current = witness.clone();
    let tag = revocation_tag(grant);
    let stop_at = acceptance_history_anchor(grant);
    let root_record = if grant.payload.issuance_action_hash.is_some() {
        Some(
            budget
                .record(grant.payload.root_action_hash.clone(), strategy)?
                .ok_or_else(|| refused("immutable root action not retrievable — PENDING"))?,
        )
    } else {
        None
    };
    let expected_type: ScopedLinkType = LinkTypes::IdToContent.try_into()?;
    for _ in 0..MAX_ACCEPTANCE_AUTHOR_HISTORY {
        budget.options(strategy)?;
        let action = current.action();
        if action.author() != &grant.payload.grantor
            || !verify_signature(action.author().clone(), current.signature().clone(), action)?
        {
            return Err(refused("acceptance author history signature differs"));
        }
        if current.action_address() == stop_at {
            if grant.payload.issuance_action_hash.is_some() {
                verify_issuance_record(
                    grant,
                    root_record
                        .as_ref()
                        .expect("v3 issuance verification fetched the root record"),
                    &current,
                )?;
            } else if current.action_address() != &grant.payload.root_action_hash {
                return Err(refused("legacy acceptance history missed immutable root"));
            }
            return Ok(());
        }
        if matches!(&action.data, ActionData::CreateLink(link)
            if link.base_address == AnyLinkableHash::from(grant.payload.root_action_hash.clone())
                && link.target_address == AnyLinkableHash::from(grant.payload.delegate.clone())
                && link.zome_index == expected_type.zome_index
                && link.link_type == expected_type.zome_type && link.tag.0 == tag)
        {
            return Err(refused("acceptance did not precede root-author revocation"));
        }
        let previous = action
            .prev_action()
            .cloned()
            .ok_or_else(|| refused("acceptance author history does not reach grant anchor"))?;
        let prior = budget
            .record(previous.clone(), strategy)?
            .ok_or_else(|| refused("acceptance author history not retrievable — PENDING"))?;
        if prior.action_address() != &previous
            || !history_predecessor_matches(
                prior.action().action_seq(),
                prior.action().timestamp(),
                action.action_seq(),
                action.timestamp(),
            )
        {
            return Err(refused("acceptance author history sequence differs"));
        }
        current = prior;
    }
    Err(refused(
        "acceptance author history budget exceeded — PENDING",
    ))
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
    let budget = AcceptanceBudget::start()?;
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
    let witness = budget
        .record(receipt.witness_action_hash.clone(), strategy)?
        .ok_or_else(|| refused("acceptance witness not retrievable — PENDING"))?;
    verify_acceptance_witness(grant, head, &witness)?;
    if receipt.accepted_at != witness.action().timestamp() {
        return Err(refused("acceptance time differs from its native witness"));
    }
    verify_acceptance_author_history(grant, &witness, strategy, &budget)?;
    // Both actions belong to the root author's source chain. Sequence order,
    // not caller timestamps, decides whether revocation preceded acceptance.
    // The v3 chain walk above sees every root-author revocation between this
    // issuance and witness, even when its link op is absent from the local index.
    // Keep the indexed legacy check for v2 compatibility only.
    if grant.payload.issuance_action_hash.is_none()
        && is_revoked(
            grant,
            strategy,
            Some(witness.action().action_seq()),
            Some(&budget),
        )?
    {
        return Err(refused("acceptance did not precede root-author revocation"));
    }
    Ok(())
}

/// Recover only a root-author accepted exact version. The signed acceptance
/// fixes its election priority; publishing the proof later cannot refresh it.
/// Resolve both root and target through conductor record lookup,
/// rather than treating the portable receipt as an ancestry substitute.
pub(crate) fn verify_accepted_publication(
    grant: &HeadDelegation,
    me: &AgentPubKey,
    id: &str,
    head: &ActionHash,
) -> ExternResult<()> {
    let root = canonical_identity_root(id, GetStrategy::Network)?
        .ok_or_else(|| refused("immutable root is not available"))?;
    let lineage = correction::get_content_lineage(correction::GetContentLineageInput {
        action_hash: head.clone(),
        local: false,
    })?;
    if lineage.truncated
        || lineage.content_id != id
        || &lineage.root_action_hash != root.action_address()
    {
        return Err(refused(
            "accepted version does not resolve to the immutable root",
        ));
    }
    verify_accepted_head(
        grant,
        me,
        root.action().author(),
        id,
        root.action_address(),
        head,
        GetStrategy::Network,
    )
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AcceptDelegatedHeadInput {
    pub id: String,
    pub head_action_hash: ActionHash,
    pub delegation: HeadDelegation,
}

fn acceptance_from_witness(
    mut input: AcceptDelegatedHeadInput,
    witness: Record,
) -> ExternResult<HeadDelegation> {
    let head = input.head_action_hash.clone();
    let mut receipt = HeadAcceptance {
        head_action_hash: input.head_action_hash,
        witness_action_hash: witness.action_address().clone(),
        accepted_at: witness.action().timestamp(),
        signature: Signature([0; 64]),
    };
    receipt.signature = sign(
        input.delegation.payload.grantor.clone(),
        &statement(&input.delegation, &receipt),
    )?;
    input.delegation.acceptance = Some(receipt);
    verify_accepted_publication(
        &input.delegation,
        &input.delegation.payload.delegate,
        &input.id,
        &head,
    )?;
    Ok(input.delegation)
}

/// Read/reconstruct an existing root-author witness only. Signing the identical
/// statement changes neither its native action nor its accepted time/priority.
#[hdk_extern]
pub fn get_accepted_delegated_head(
    input: AcceptDelegatedHeadInput,
) -> ExternResult<Option<HeadDelegation>> {
    let author = agent_info()?.agent_initial_pubkey;
    if author != input.delegation.payload.grantor {
        return Err(refused("only root author may retrieve acceptance"));
    }
    let root = canonical_identity_root(&input.id, GetStrategy::Network)?
        .ok_or_else(|| refused("immutable root is not available"))?;
    verify_grant(
        &input.delegation,
        &input.delegation.payload.delegate,
        root.action().author(),
        &input.id,
        root.action_address(),
        GetStrategy::Network,
    )?;
    let witness = witnessed_acceptance(
        &input.delegation,
        &input.head_action_hash,
        GetStrategy::Network,
    )?;
    witness
        .map(|witness| acceptance_from_witness(input, witness))
        .transpose()
}

#[hdk_extern]
pub fn accept_delegated_head(input: AcceptDelegatedHeadInput) -> ExternResult<HeadDelegation> {
    let author = agent_info()?.agent_initial_pubkey;
    if author != input.delegation.payload.grantor {
        return Err(refused("only root author may accept"));
    }
    let lookup = AcceptDelegatedHeadInput {
        id: input.id.clone(),
        head_action_hash: input.head_action_hash.clone(),
        delegation: input.delegation.clone(),
    };
    if let Some(prior) = get_accepted_delegated_head(lookup)? {
        return Ok(prior);
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
    if lineage.truncated
        || lineage.content_id != input.id
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
    let witness = create_link(
        input.delegation.payload.root_action_hash.clone(),
        input.head_action_hash.clone(),
        LinkTypes::IdToContent,
        LinkTag::new(acceptance_tag(&input.delegation, &input.head_action_hash)),
    )?;
    let record = get(witness, GetOptions::local())?
        .ok_or_else(|| refused("acceptance witness not retrievable — PENDING"))?;
    verify_acceptance_witness(&input.delegation, &input.head_action_hash, &record)?;
    acceptance_from_witness(input, record)
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
                issuance_action_hash: None,
            },
            signature: Signature([5; 64]),
            acceptance: Some(HeadAcceptance {
                witness_action_hash: ActionHash::from_raw_36(vec![8; 36]),
                head_action_hash: ActionHash::from_raw_36(vec![6; 36]),
                accepted_at: Timestamp::from_micros(99),
                signature: Signature([7; 64]),
            }),
        }
    }

    #[test]
    fn acceptance_reads_spend_one_budget_and_stop_before_another_host_call() {
        let start = Timestamp::from_micros(10_000_000);
        let at = |ms: i64| Timestamp::from_micros(10_000_000 + ms * 1000);
        let first = acceptance_get_options(start, at(0), GetStrategy::Network).unwrap();
        assert_eq!(first.timeout_ms(), Some(u64::from(BATCH_BUDGET_DEFAULT_MS)));
        let next = acceptance_get_options(start, at(3_999), GetStrategy::Network).unwrap();
        assert_eq!(next.timeout_ms(), Some(1));
        assert_eq!(next.strategy(), GetStrategy::Network);
        assert!(acceptance_get_options(start, at(4_000), GetStrategy::Network).is_none());
        assert!(acceptance_get_options(start, at(60_000), GetStrategy::Local).is_none());
        let local = acceptance_get_options(start, at(1), GetStrategy::Local).unwrap();
        assert_eq!(local.strategy(), GetStrategy::Local);
        // Backward wall-clock steps never grant more than the original budget;
        // the independent record cap remains effective too.
        assert_eq!(
            acceptance_get_options(start, at(-1), GetStrategy::Network)
                .unwrap()
                .timeout_ms(),
            first.timeout_ms()
        );
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
    fn legacy_v2_payload_and_acceptance_bytes_remain_unchanged() {
        #[derive(Debug, Serialize)]
        struct LegacyPayload {
            grantor: AgentPubKey,
            delegate: AgentPubKey,
            scope: String,
            valid_until: Timestamp,
            root_action_hash: ActionHash,
            dna_hash: DnaHash,
        }
        #[derive(Debug, Serialize)]
        struct LegacyStatement<'a> {
            domain: &'static str,
            grant: &'a LegacyPayload,
            grant_signature: &'a Signature,
            head_action_hash: &'a ActionHash,
            witness_action_hash: &'a ActionHash,
            accepted_at: Timestamp,
        }

        let g = grant();
        let legacy = LegacyPayload {
            grantor: g.payload.grantor.clone(),
            delegate: g.payload.delegate.clone(),
            scope: g.payload.scope.clone(),
            valid_until: g.payload.valid_until,
            root_action_hash: g.payload.root_action_hash.clone(),
            dna_hash: g.payload.dna_hash.clone(),
        };
        let actual_payload = holochain_serialized_bytes::encode(&g.payload).unwrap();
        let legacy_payload = holochain_serialized_bytes::encode(&legacy).unwrap();
        assert_eq!(actual_payload, legacy_payload);

        let receipt = g.acceptance.as_ref().unwrap();
        let old_statement = LegacyStatement {
            domain: "elohim:accepted-content-head:v2",
            grant: &legacy,
            grant_signature: &g.signature,
            head_action_hash: &receipt.head_action_hash,
            witness_action_hash: &receipt.witness_action_hash,
            accepted_at: receipt.accepted_at,
        };
        assert_eq!(
            holochain_serialized_bytes::encode(&statement(&g, receipt)).unwrap(),
            holochain_serialized_bytes::encode(&old_statement).unwrap()
        );
    }

    #[test]
    fn new_grant_history_is_anchored_at_its_unique_issuance_action() {
        let mut g = grant();
        let ancient_root = g.payload.root_action_hash.clone();
        let issuance = ActionHash::from_raw_36(vec![9; 36]);
        g.payload.issuance_action_hash = Some(issuance.clone());
        assert_eq!(acceptance_history_anchor(&g), &issuance);
        assert_ne!(acceptance_history_anchor(&g), &ancient_root);

        let tag = revocation_tag(&g);
        let mut another_issuance = g.clone();
        another_issuance.payload.issuance_action_hash = Some(ActionHash::from_raw_36(vec![10; 36]));
        assert_ne!(tag, revocation_tag(&another_issuance));
        assert_ne!(tag, revocation_tag(&grant()));
    }

    #[test]
    fn short_issuer_history_is_valid_even_when_the_content_root_is_ancient() {
        let root_sequence = 319;
        let issuance_sequence = 29_502;
        let witness_sequence = 29_504;
        let root_at = Timestamp::from_micros(1_000);
        let issuance_at = Timestamp::from_micros(2_000);
        let revocation_at = Timestamp::from_micros(3_000);
        let witness_at = Timestamp::from_micros(4_000);

        assert!(root_sequence < issuance_sequence);
        assert!(root_at < issuance_at);
        assert!(history_predecessor_matches(
            issuance_sequence,
            issuance_at,
            issuance_sequence + 1,
            revocation_at,
        ));
        assert!(history_predecessor_matches(
            issuance_sequence + 1,
            revocation_at,
            witness_sequence,
            witness_at,
        ));
        assert!(!history_predecessor_matches(
            issuance_sequence,
            issuance_at,
            witness_sequence,
            witness_at,
        ));
    }

    fn signed_test_record(action: Action) -> Record {
        let hash = ActionHash::with_data_sync(&action);
        Record::new(
            SignedActionHashed::with_presigned(
                ActionHashed::with_pre_hashed(action, hash),
                Signature([0; 64]),
            ),
            RecordEntry::NotStored,
        )
    }

    fn issuance_records(
        root_action: Action,
        issuer_action: Action,
        grant_root: Option<ActionHash>,
        grant_delegate: AgentPubKey,
        valid_until: Timestamp,
    ) -> (HeadDelegation, Record, Record) {
        let root = signed_test_record(root_action);
        let root_hash = grant_root.unwrap_or_else(|| root.action_address().clone());
        let issuer = signed_test_record(issuer_action);
        let grant = HeadDelegation {
            payload: HeadDelegationPayload {
                grantor: root.action().author().clone(),
                delegate: grant_delegate,
                scope: "lesson-1".into(),
                valid_until,
                root_action_hash: root_hash,
                dna_hash: DnaHash::from_raw_36(vec![4; 36]),
                issuance_action_hash: Some(issuer.action_address().clone()),
            },
            signature: Signature([1; 64]),
            acceptance: None,
        };
        (grant, root, issuer)
    }

    fn issuance_fixture() -> (Action, Action, AgentPubKey, Timestamp, ScopedLinkType) {
        let grantor = AgentPubKey::from_raw_36(vec![1; 36]);
        let delegate = AgentPubKey::from_raw_36(vec![2; 36]);
        let valid_until = Timestamp::from_micros(100_000);
        let root = Action {
            header: ActionHeader {
                author: grantor.clone(),
                timestamp: Timestamp::from_micros(1_000),
                action_seq: 319,
                prev_action: Some(ActionHash::from_raw_36(vec![3; 36])),
            },
            data: ActionData::Create(CreateData {
                entry_type: EntryType::App(AppEntryDef::new(
                    EntryDefIndex(0),
                    ZomeIndex(0),
                    EntryVisibility::Public,
                )),
                entry_hash: EntryHash::from_raw_36(vec![5; 36]),
            }),
        };
        let root_hash = ActionHash::with_data_sync(&root);
        // IdToContent is the first link in this sole zome's `#[hdk_link_types]`
        // declaration. Unit tests have no HDK zome scope registry to query.
        let link_type = ScopedLinkType {
            zome_index: 0.into(),
            zome_type: 0.into(),
        };
        let issuer = Action {
            header: ActionHeader {
                author: grantor,
                timestamp: Timestamp::from_micros(2_000),
                action_seq: 29_502,
                prev_action: Some(ActionHash::from_raw_36(vec![6; 36])),
            },
            data: ActionData::CreateLink(CreateLinkData {
                base_address: AnyLinkableHash::from(root_hash),
                target_address: AnyLinkableHash::from(delegate.clone()),
                zome_index: link_type.zome_index,
                link_type: link_type.zome_type,
                tag: LinkTag::new(issuance_tag(valid_until)),
            }),
        };
        (root, issuer, delegate, valid_until, link_type)
    }

    #[test]
    fn native_issuance_record_rejects_wrong_author_root_delegate_tag_and_link_type() {
        let (root_action, issuer_action, delegate, valid_until, expected_link_type) =
            issuance_fixture();
        let (valid, root, record) = issuance_records(
            root_action.clone(),
            issuer_action.clone(),
            None,
            delegate.clone(),
            valid_until,
        );
        assert!(issuance_context_matches(
            &valid,
            &root,
            &record,
            expected_link_type
        ));

        let mut wrong_author = issuer_action.clone();
        wrong_author.header.author = AgentPubKey::from_raw_36(vec![8; 36]);
        let (grant, root, record) = issuance_records(
            root_action.clone(),
            wrong_author,
            None,
            delegate.clone(),
            valid_until,
        );
        assert!(!issuance_context_matches(
            &grant,
            &root,
            &record,
            expected_link_type
        ));

        let wrong_base = ActionHash::from_raw_36(vec![9; 36]);
        let mut wrong_root_link = issuer_action.clone();
        if let ActionData::CreateLink(link) = &mut wrong_root_link.data {
            link.base_address = AnyLinkableHash::from(wrong_base.clone());
        }
        let (grant, root, record) = issuance_records(
            root_action.clone(),
            wrong_root_link,
            None,
            delegate.clone(),
            valid_until,
        );
        assert!(!issuance_context_matches(
            &grant,
            &root,
            &record,
            expected_link_type
        ));

        let mut issuer_for_wrong_grant_root = issuer_action.clone();
        if let ActionData::CreateLink(link) = &mut issuer_for_wrong_grant_root.data {
            link.base_address = AnyLinkableHash::from(wrong_base.clone());
        }
        let (grant, root, record) = issuance_records(
            root_action.clone(),
            issuer_for_wrong_grant_root,
            Some(wrong_base),
            delegate.clone(),
            valid_until,
        );
        assert!(!issuance_context_matches(
            &grant,
            &root,
            &record,
            expected_link_type
        ));

        let mut wrong_delegate = issuer_action.clone();
        if let ActionData::CreateLink(link) = &mut wrong_delegate.data {
            link.target_address = AnyLinkableHash::from(AgentPubKey::from_raw_36(vec![10; 36]));
        }
        let (grant, root, record) = issuance_records(
            root_action.clone(),
            wrong_delegate,
            None,
            delegate.clone(),
            valid_until,
        );
        assert!(!issuance_context_matches(
            &grant,
            &root,
            &record,
            expected_link_type
        ));

        let mut wrong_tag = issuer_action.clone();
        if let ActionData::CreateLink(link) = &mut wrong_tag.data {
            link.tag = LinkTag::new(issuance_tag(Timestamp::from_micros(99_999)));
        }
        let (grant, root, record) = issuance_records(
            root_action.clone(),
            wrong_tag,
            None,
            delegate.clone(),
            valid_until,
        );
        assert!(!issuance_context_matches(
            &grant,
            &root,
            &record,
            expected_link_type
        ));

        for (zome_index, link_type) in [(1.into(), 0.into()), (0.into(), 1.into())] {
            let mut wrong_type = issuer_action.clone();
            if let ActionData::CreateLink(link) = &mut wrong_type.data {
                link.zome_index = zome_index;
                link.link_type = link_type;
            }
            let (grant, root, record) = issuance_records(
                root_action.clone(),
                wrong_type,
                None,
                delegate.clone(),
                valid_until,
            );
            assert!(!issuance_context_matches(
                &grant,
                &root,
                &record,
                expected_link_type
            ));
        }
    }

    #[test]
    fn native_issuance_record_must_follow_root_and_precede_expiry() {
        let (root_action, issuer_action, delegate, valid_until, expected_link_type) =
            issuance_fixture();
        for (action_seq, timestamp, should_match) in [
            (318, 2_000, false),
            (29_502, 1_000, false),
            (29_502, 100_000, false),
            (29_502, 2_000, true),
        ] {
            let mut issuer = issuer_action.clone();
            issuer.header.action_seq = action_seq;
            issuer.header.timestamp = Timestamp::from_micros(timestamp);
            let (grant, root, record) = issuance_records(
                root_action.clone(),
                issuer,
                None,
                delegate.clone(),
                valid_until,
            );
            assert_eq!(
                issuance_context_matches(&grant, &root, &record, expected_link_type),
                should_match,
                "seq={action_seq}, timestamp={timestamp}"
            );
        }
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
