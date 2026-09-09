//! Bounded observation of an existing REA Commitment's signed Update graph.
//! This is not a claim of global freshness, provider authority, or fork absence.

use content_store_integrity::{Commitment, EntryTypes, LinkTypes, StringAnchor, UnitEntryTypes};
use hdk::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::correction::{MAX_LINEAGE_CANDIDATES, MAX_LINEAGE_DEPTH};

#[derive(Clone)]
pub(crate) struct Observation {
    pub record: Record,
    pub commitment: Commitment,
    updates: Vec<ActionHash>,
}

fn unavailable(reason: &str) -> WasmError {
    wasm_error!(WasmErrorInner::Guest(format!(
        "commitment observation unavailable: {reason}"
    )))
}

fn read_observation(hash: &ActionHash) -> ExternResult<Observation> {
    let Some(Details::Record(details)) = get_details(hash.clone(), GetOptions::default())? else {
        return Err(unavailable("record details missing"));
    };
    if details.validation_status != ValidationStatus::Valid {
        return Err(unavailable("record is not validated"));
    }
    let record = details.record;
    let expected: EntryType = UnitEntryTypes::Commitment.try_into()?;
    if record.action_address() != hash || record.action().entry_type() != Some(&expected) {
        return Err(unavailable("wrong action or entry type"));
    }
    let commitment: Commitment = record
        .entry()
        .to_app_option()
        .map_err(|_| unavailable("malformed Commitment"))?
        .ok_or_else(|| unavailable("Commitment bytes missing"))?;
    let entry_hash = hash_entry(&EntryTypes::Commitment(commitment.clone()))?;
    if record.action().entry_hash() != Some(&entry_hash) {
        return Err(unavailable("entry hash mismatch"));
    }
    // No silent rollback to a pre-delete version. Deletion semantics are not
    // lifecycle-state semantics and require a separate disposition.
    if !details.deletes.is_empty() {
        return Err(unavailable("observed delete"));
    }
    if details.updates.len() > MAX_LINEAGE_CANDIDATES {
        return Err(unavailable("update metadata budget exceeded"));
    }
    Ok(Observation {
        record,
        commitment,
        updates: details
            .updates
            .into_iter()
            .map(|update| update.action_address().clone())
            .collect(),
    })
}

fn same_undertaking(a: &Commitment, b: &Commitment) -> bool {
    let mut normalized = b.clone();
    normalized.state.clone_from(&a.state);
    normalized.finished = a.finished;
    normalized.updated_at.clone_from(&a.updated_at);
    a == &normalized
}

fn load<F>(
    hash: &ActionHash,
    cache: &mut HashMap<ActionHash, Observation>,
    read: &mut F,
) -> ExternResult<Observation>
where
    F: FnMut(&ActionHash) -> ExternResult<Observation>,
{
    if let Some(record) = cache.get(hash) {
        return Ok(record.clone());
    }
    // One total distinct-record budget covers root AND forward walks.
    if cache.len() >= MAX_LINEAGE_CANDIDATES {
        return Err(unavailable("record budget exceeded"));
    }
    let record = read(hash)?;
    if record.record.action_address() != hash {
        return Err(unavailable("record action mismatch"));
    }
    cache.insert(hash.clone(), record.clone());
    Ok(record)
}

fn observe_with<F>(
    id: &str,
    targets: Vec<ActionHash>,
    mut read: F,
) -> ExternResult<Option<Observation>>
where
    F: FnMut(&ActionHash) -> ExternResult<Observation>,
{
    let mut cache = HashMap::new();
    let mut root: Option<ActionHash> = None;
    let mut distinct_targets = HashSet::new();
    for target in targets {
        if !distinct_targets.insert(target.clone()) {
            continue;
        }
        let mut current = target;
        let mut ancestors = HashSet::new();
        loop {
            if ancestors.len() >= MAX_LINEAGE_DEPTH || !ancestors.insert(current.clone()) {
                return Err(unavailable("cyclic or over-depth lineage"));
            }
            let observed = load(&current, &mut cache, &mut read)?;
            if observed.commitment.id != id {
                return Err(unavailable("anchor target has another commitment ID"));
            }
            match &observed.record.action().data {
                ActionData::Create(_) => {
                    if root.as_ref().is_some_and(|known| known != &current) {
                        return Err(unavailable("multiple root Creates for ID"));
                    }
                    root = Some(current);
                    break;
                }
                ActionData::Update(update) => current = update.original_action_address.clone(),
                _ => return Err(unavailable("root is not a Create")),
            }
        }
    }
    let Some(root_hash) = root else {
        return Ok(None);
    };
    let original = load(&root_hash, &mut cache, &mut read)?;
    // Own-chain queries can already name an Update before its reverse metadata
    // arrives. Check its claimed predecessor edge in the same forward walk.
    let mut known_children: HashMap<ActionHash, Vec<ActionHash>> = HashMap::new();
    for (hash, observation) in &cache {
        if let ActionData::Update(update) = &observation.record.action().data {
            known_children
                .entry(update.original_action_address.clone())
                .or_default()
                .push(hash.clone());
        }
    }
    let mut selected = original.clone();
    let mut sequences = HashMap::from([(original.record.action().action_seq(), root_hash.clone())]);
    let mut visited = HashSet::new();
    let mut scheduled = HashSet::from([root_hash.clone()]);
    let mut pending = VecDeque::from([(root_hash, 0usize)]);
    while let Some((hash, depth)) = pending.pop_front() {
        if !visited.insert(hash.clone()) {
            continue;
        }
        if depth >= MAX_LINEAGE_DEPTH {
            return Err(unavailable("update depth exceeded"));
        }
        let parent = load(&hash, &mut cache, &mut read)?;
        let mut children = parent.updates.clone();
        if let Some(known) = known_children.get(&hash) {
            children.extend(known.iter().cloned());
        }
        children.sort();
        children.dedup();
        for next in &children {
            let candidate = load(next, &mut cache, &mut read)?;
            // A foreign or changed undertaking confers no lifecycle state.
            // Do not follow its descendants. Reading it still consumes the
            // shared budget and can expose an unavailable record.
            if candidate.record.action().author() != original.record.action().author()
                || !same_undertaking(&original.commitment, &candidate.commitment)
            {
                continue;
            }
            let ActionData::Update(update) = &candidate.record.action().data else {
                return Err(unavailable("update metadata targets a non-Update"));
            };
            if update.original_action_address != hash
                || parent.record.action().entry_hash() != Some(&update.original_entry_address)
                || candidate.record.action().action_seq() <= parent.record.action().action_seq()
            {
                return Err(unavailable(
                    "invalid update predecessor binding or sequence",
                ));
            }
            let seq = candidate.record.action().action_seq();
            if sequences
                .insert(seq, next.clone())
                .is_some_and(|old| old != *next)
            {
                return Err(unavailable("conflicting actions at one signed sequence"));
            }
            if seq > selected.record.action().action_seq() {
                selected = candidate;
            }
            if scheduled.insert(next.clone()) {
                pending.push_back((next.clone(), depth + 1));
            }
        }
    }
    Ok(Some(selected))
}

pub(crate) fn observe(id: &str) -> ExternResult<Option<Observation>> {
    let anchor = hash_entry(&EntryTypes::StringAnchor(StringAnchor::new(
        "commitment_id",
        id,
    )))?;
    let links = get_links(
        LinkQuery::try_new(anchor, LinkTypes::IdToCommitment)?,
        GetStrategy::default(),
    )?;
    let targets = links
        .into_iter()
        .map(|link| {
            ActionHash::try_from(link.target).map_err(|_| unavailable("non-action anchor target"))
        })
        .collect::<ExternResult<Vec<_>>>()?;
    observe_targets(id, targets)
}

pub(crate) fn observe_targets(
    id: &str,
    targets: Vec<ActionHash>,
) -> ExternResult<Option<Observation>> {
    observe_with(id, targets, read_observation)
}

pub(crate) fn output(observed: Observation) -> ExternResult<shefa_types::ReaCommitmentOutput> {
    Ok(shefa_types::ReaCommitmentOutput {
        action_hash: observed.record.action_address().clone(),
        entry_hash: observed
            .record
            .action()
            .entry_hash()
            .cloned()
            .ok_or_else(|| unavailable("selected record has no entry hash"))?,
        commitment: crate::commitment_to_wire(&observed.commitment),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ah(n: u8) -> ActionHash {
        ActionHash::from_raw_32(vec![n; 32])
    }
    fn eh(n: u8) -> EntryHash {
        EntryHash::from_raw_32(vec![n; 32])
    }

    // These fixtures enter AFTER the notary loader. They exercise graph policy,
    // not cryptographic validation; real signed readback is covered by Sweettest.
    fn node(n: u8, parent: Option<u8>, seq: u32, state: &str, updates: &[u8]) -> Observation {
        let commitment = Commitment {
            id: "undertaking".into(),
            action: "provide".into(),
            provider: "provider".into(),
            receiver: "household".into(),
            resource_conforms_to: None,
            resource_inventoried_as: None,
            resource_classified_as_json: "[\"content:commons\"]".into(),
            resource_quantity_value: None,
            resource_quantity_unit: None,
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_point_in_time: None,
            has_beginning: None,
            has_end: None,
            due: None,
            clause_of: Some("agreement".into()),
            agreed_in: None,
            input_of: None,
            output_of: None,
            satisfies: None,
            in_scope_of_json: "[\"epr:lamad\"]".into(),
            finished: state == "cancelled",
            state: state.into(),
            note: None,
            metadata_json: "{}".into(),
            created_at: "created".into(),
            updated_at: "untrusted timestamp".into(),
        };
        let entry_type = EntryType::App(AppEntryDef::new(
            EntryDefIndex(0),
            ZomeIndex(0),
            EntryVisibility::Public,
        ));
        let data = match parent {
            None => ActionData::Create(CreateData {
                entry_type,
                entry_hash: eh(n),
            }),
            Some(p) => ActionData::Update(UpdateData {
                original_action_address: ah(p),
                original_entry_address: eh(p),
                entry_type,
                entry_hash: eh(n),
            }),
        };
        let action = Action {
            header: ActionHeader {
                author: AgentPubKey::from_raw_32(vec![1; 32]),
                timestamp: Timestamp::from_micros(1),
                action_seq: seq,
                prev_action: None,
            },
            data,
        };
        Observation {
            record: Record::new(
                SignedActionHashed::with_presigned(
                    ActionHashed::with_pre_hashed(action, ah(n)),
                    Signature([0; 64]),
                ),
                RecordEntry::NotStored,
            ),
            commitment,
            updates: updates.iter().copied().map(ah).collect(),
        }
    }

    fn resolve(nodes: Vec<Observation>, targets: &[u8]) -> ExternResult<Option<Observation>> {
        let map: HashMap<_, _> = nodes
            .into_iter()
            .map(|n| (n.record.action_address().clone(), n))
            .collect();
        observe_with(
            "undertaking",
            targets.iter().copied().map(ah).collect(),
            |hash| {
                map.get(hash)
                    .cloned()
                    .ok_or_else(|| unavailable("fixture missing"))
            },
        )
    }

    #[test]
    fn observed_commitment_lifecycle_follows_updates_and_legacy_siblings() {
        for parent in [1, 2] {
            let root_updates = if parent == 1 { vec![3, 2, 2] } else { vec![2] };
            let second_updates = if parent == 2 { vec![3] } else { vec![] };
            let result = resolve(
                vec![
                    node(1, None, 1, "created", &root_updates),
                    node(2, Some(1), 5, "active", &second_updates),
                    node(3, Some(parent), 99, "cancelled", &[]),
                ],
                &[1, 1, 2],
            )
            .unwrap()
            .unwrap();
            assert_eq!(result.record.action_address(), &ah(3));
            assert_eq!(result.record.action().entry_hash(), Some(&eh(3)));
            assert_eq!(result.commitment.state, "cancelled");
            assert!(result.commitment.finished);
        }
    }

    #[test]
    fn observed_commitment_foreign_or_changed_undertakings_confer_no_state() {
        for changed_field in ["author", "provider", "scope", "id"] {
            let mut forged = node(3, Some(1), 99, "active", &[]);
            match changed_field {
                "author" => {
                    let mut action = forged.record.action().clone();
                    action.header.author = AgentPubKey::from_raw_32(vec![2; 32]);
                    forged.record = Record::new(
                        SignedActionHashed::with_presigned(
                            ActionHashed::with_pre_hashed(action, ah(3)),
                            Signature([0; 64]),
                        ),
                        RecordEntry::NotStored,
                    );
                }
                "provider" => forged.commitment.provider = "stranger".into(),
                "scope" => forged.commitment.in_scope_of_json = "[\"all\"]".into(),
                _ => forged.commitment.id = "other".into(),
            }
            let result = resolve(
                vec![
                    node(1, None, 1, "created", &[3, 2]),
                    node(2, Some(1), 2, "cancelled", &[]),
                    forged,
                ],
                &[1],
            )
            .unwrap()
            .unwrap();
            assert_eq!(result.record.action_address(), &ah(2), "{changed_field}");
        }
    }

    #[test]
    fn observed_commitment_conflicts_missing_records_and_invalid_edges_refuse() {
        assert!(resolve(
            vec![
                node(1, None, 1, "created", &[]),
                node(2, None, 2, "active", &[])
            ],
            &[1, 2]
        )
        .is_err());
        assert!(resolve(vec![node(1, None, 1, "created", &[2])], &[1]).is_err());
        for seq in [1, 2] {
            assert!(resolve(
                vec![
                    node(1, None, 1, "created", &[2, 3]),
                    node(2, Some(1), 2, "active", &[]),
                    node(3, Some(1), seq, "cancelled", &[])
                ],
                &[1]
            )
            .is_err());
        }
        let mut wrong_entry = node(2, Some(1), 2, "active", &[]);
        let mut action = wrong_entry.record.action().clone();
        if let ActionData::Update(update) = &mut action.data {
            update.original_entry_address = eh(99);
        }
        wrong_entry.record = Record::new(
            SignedActionHashed::with_presigned(
                ActionHashed::with_pre_hashed(action, ah(2)),
                Signature([0; 64]),
            ),
            RecordEntry::NotStored,
        );
        assert!(resolve(
            vec![node(1, None, 1, "created", &[2]), wrong_entry.clone()],
            &[1]
        )
        .is_err());
        // A source-chain query's positive Update must pass the same binding
        // check even before the root's reverse update metadata arrives.
        assert!(resolve(vec![node(1, None, 1, "created", &[]), wrong_entry], &[1, 2]).is_err());
    }

    #[test]
    fn observed_commitment_record_budget_is_total_and_missing_updates_are_not_revocation_proof() {
        let mut nodes = vec![node(1, None, 1, "created", &[2])];
        for n in 2..=65 {
            let next = if n < 65 { vec![n + 1] } else { vec![] };
            nodes.push(node(n, Some(n - 1), n as u32, "active", &next));
        }
        assert!(resolve(nodes, &[1]).is_err());
        let result = resolve(vec![node(1, None, 1, "created", &[])], &[1])
            .unwrap()
            .unwrap();
        assert_eq!(result.commitment.state, "created");
        assert!(resolve(vec![], &[]).unwrap().is_none());
    }

    #[test]
    fn observed_commitment_own_chain_update_does_not_wait_for_reverse_metadata() {
        let result = resolve(
            vec![
                node(1, None, 1, "created", &[]),
                node(2, Some(1), 2, "cancelled", &[]),
            ],
            &[1, 2],
        )
        .unwrap()
        .unwrap();
        assert_eq!(result.commitment.state, "cancelled");
    }
}
