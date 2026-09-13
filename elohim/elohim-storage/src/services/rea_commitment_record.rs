//! Authenticate a bounded pair of REA lifecycle lineages from own lamad records.
//! No revocation, global freshness, fork-absence or provider-authority claim.
use super::{
    authenticated_record::{verify_record, RecordRequirements},
    conductor_writes::CarriedRecordWire,
};
use crate::error::StorageError;
use holochain_types::prelude::*;
use shefa_types::Commitment;
use std::{collections::HashMap, future::Future};

// Matches content_store::correction's existing lineage depth/candidate bound.
const MAX_RECORDS: usize = 64;
const MAX_RECORD_BYTES: usize = super::commitment_record::MAX_COMMITMENT_RECORD_BYTES;
// content_store_integrity::EntryTypes::Commitment, current lamad integrity zome.
const COMMITMENT_ENTRY: u8 = 35;

#[derive(Clone)]
struct Version {
    record: Record,
    commitment: Commitment,
}

pub(crate) struct Observation {
    pub commitment: Commitment,
    pub action_hash: String,
}

fn invalid(reason: &str) -> StorageError {
    StorageError::InvalidInput(format!("REA lifecycle observation unavailable: {reason}"))
}

fn same_undertaking(a: &Commitment, b: &Commitment) -> Result<bool, StorageError> {
    let mut normalized = b.clone();
    normalized.state.clone_from(&a.state);
    normalized.finished = a.finished;
    normalized.updated_at.clone_from(&a.updated_at);
    Ok(
        rmp_serde::to_vec_named(a).map_err(|_| invalid("entry encoding"))?
            == rmp_serde::to_vec_named(&normalized).map_err(|_| invalid("entry encoding"))?,
    )
}

async fn load<F, Fut>(
    hash: &ActionHash,
    cache: &mut HashMap<ActionHash, Version>,
    read: &mut F,
) -> Result<Version, StorageError>
where
    F: FnMut(String) -> Fut,
    Fut: Future<Output = Result<Option<CarriedRecordWire>, StorageError>>,
{
    if let Some(v) = cache.get(hash) {
        return Ok(v.clone());
    }
    if cache.len() >= MAX_RECORDS {
        return Err(invalid("64-record budget exceeded"));
    }
    let wire = read(hash.to_string())
        .await?
        .ok_or_else(|| invalid("record missing"))?;
    if wire.action_hash != hash.to_string() {
        return Err(invalid("response anchor mismatch"));
    }
    let record = verify_record(
        &wire.record,
        RecordRequirements {
            action_hash: hash,
            entry_hash: None,
            author: None,
            zome_index: 0,
            entry_index: COMMITMENT_ENTRY,
            allow_update: true,
            max_bytes: MAX_RECORD_BYTES,
        },
    )?
    .ok_or_else(|| invalid("record missing"))?;
    let Some(Entry::App(app)) = record.entry().as_option() else {
        return Err(invalid("entry missing"));
    };
    let commitment: Commitment =
        rmp_serde::from_slice(app.bytes()).map_err(|_| invalid("malformed Commitment"))?;
    let v = Version { record, commitment };
    cache.insert(hash.clone(), v.clone());
    Ok(v)
}

async fn root<F, Fut>(
    first: &Version,
    id: &str,
    cache: &mut HashMap<ActionHash, Version>,
    read: &mut F,
) -> Result<ActionHash, StorageError>
where
    F: FnMut(String) -> Fut,
    Fut: Future<Output = Result<Option<CarriedRecordWire>, StorageError>>,
{
    let mut current = first.clone();
    // Each edge strictly decreases signed sequence, and total distinct reads are
    // capped before the local exact-record call; no abandoned WASM scan timeout.
    for _ in 0..MAX_RECORDS {
        if current.commitment.id != id || !same_undertaking(&first.commitment, &current.commitment)?
        {
            return Err(invalid("different undertaking"));
        }
        match &current.record.action().data {
            ActionData::Create(_) => return Ok(current.record.action_address().clone()),
            ActionData::Update(update) => {
                let parent = load(&update.original_action_address, cache, read).await?;
                if parent.record.action().author() != first.record.action().author()
                    || parent.record.action().action_seq() >= current.record.action().action_seq()
                    || parent.record.action().timestamp() > current.record.action().timestamp()
                    || parent.record.action().entry_hash() != Some(&update.original_entry_address)
                {
                    return Err(invalid("invalid signed predecessor edge"));
                }
                current = parent;
            }
            _ => return Err(invalid("expected Create/Update")),
        }
    }
    Err(invalid("64-edge depth exceeded"))
}

/// None means a same/older observed version; unavailable evidence is an error,
/// never an instruction to insert or replace a row.
pub(crate) async fn observe<F, Fut>(
    id: &str,
    incoming: &str,
    stored: Option<&str>,
    mut read: F,
) -> Result<Option<Observation>, StorageError>
where
    F: FnMut(String) -> Fut,
    Fut: Future<Output = Result<Option<CarriedRecordWire>, StorageError>>,
{
    let parse = |s: &str| ActionHash::try_from(s).map_err(|_| invalid("malformed action hash"));
    let mut cache = HashMap::new();
    let incoming_hash = parse(incoming)?;
    let next = load(&incoming_hash, &mut cache, &mut read).await?;
    let next_root = root(&next, id, &mut cache, &mut read).await?;
    if let Some(previous) = stored {
        let old_hash = parse(previous)?;
        let old = load(&old_hash, &mut cache, &mut read).await?;
        let old_root = root(&old, id, &mut cache, &mut read).await?;
        if next_root != old_root
            || next.record.action().author() != old.record.action().author()
            || !same_undertaking(&next.commitment, &old.commitment)?
        {
            return Err(invalid("different root, author or undertaking"));
        }
        if next.record.action().action_seq() == old.record.action().action_seq()
            && incoming_hash != old_hash
        {
            return Err(invalid("ambiguous signed sequence"));
        }
        if next.record.action().action_seq() <= old.record.action().action_seq() {
            return Ok(None);
        }
        if next.record.action().timestamp() < old.record.action().timestamp() {
            return Err(invalid("signed time regressed"));
        }
    }
    Ok(Some(Observation {
        commitment: next.commitment,
        action_hash: incoming.to_string(),
    }))
}

#[cfg(test)]
#[path = "rea_commitment_record_tests.rs"]
mod tests;
