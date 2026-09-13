//! Shared bounded authentication of exact own-conductor records.
//! Entity meaning, lineage, and authorization remain with each caller.
use ed25519_dalek::VerifyingKey;
use holochain_types::prelude::*;

use crate::error::StorageError;

pub(crate) struct RecordRequirements<'a> {
    pub action_hash: &'a ActionHash,
    pub entry_hash: Option<&'a EntryHash>,
    pub author: Option<&'a AgentPubKey>,
    pub zome_index: u8,
    pub entry_index: u8,
    pub allow_update: bool,
    pub max_bytes: usize,
}

fn invalid(reason: &str) -> StorageError {
    StorageError::InvalidInput(format!("authenticated record: {reason}"))
}

/// Only accepts public application Create/Update records from the pinned entry
/// definition. Recomputes both hashes and verifies the actual author's action
/// signature; this does not imply that author may speak for another party.
pub(crate) fn verify_record(
    bytes: &[u8],
    pins: RecordRequirements<'_>,
) -> Result<Option<Record>, StorageError> {
    if bytes.len() > pins.max_bytes {
        return Err(invalid("record exceeds byte limit"));
    }
    let record: Option<Record> =
        rmp_serde::from_slice(bytes).map_err(|_| invalid("malformed record"))?;
    let Some(record) = record else {
        return Ok(None);
    };
    if !(matches!(record.action().data, ActionData::Create(_))
        || pins.allow_update && matches!(record.action().data, ActionData::Update(_)))
    {
        return Err(invalid("unexpected action kind"));
    }
    let Some(EntryType::App(def)) = record.action().entry_type() else {
        return Err(invalid("expected app entry"));
    };
    if def.zome_index != ZomeIndex(pins.zome_index)
        || def.entry_index != EntryDefIndex(pins.entry_index)
        || def.visibility != EntryVisibility::Public
    {
        return Err(invalid("wrong entry definition"));
    }
    if pins
        .author
        .is_some_and(|author| record.action().author() != author)
    {
        return Err(invalid("author pin mismatch"));
    }
    let Some(entry @ Entry::App(app)) = record.entry().as_option() else {
        return Err(invalid("missing app entry"));
    };
    if app.bytes().len() > pins.max_bytes {
        return Err(invalid("entry exceeds byte limit"));
    }
    let action_hash = ActionHash::with_data_sync(record.action());
    if &action_hash != pins.action_hash || record.action_address() != &action_hash {
        return Err(invalid("action pin mismatch"));
    }
    let entry_hash = EntryHash::with_data_sync(entry);
    if pins
        .entry_hash
        .is_some_and(|expected| expected != &entry_hash)
        || record.action().entry_hash() != Some(&entry_hash)
    {
        return Err(invalid("entry pin mismatch"));
    }
    let action_bytes = holochain_types::prelude::encode(record.action())
        .map_err(|_| invalid("action encoding failed"))?;
    let key_bytes: [u8; 32] = record
        .action()
        .author()
        .get_raw_32()
        .try_into()
        .map_err(|_| invalid("malformed author key"))?;
    let key = VerifyingKey::from_bytes(&key_bytes).map_err(|_| invalid("invalid author key"))?;
    let signature = ed25519_dalek::Signature::from_slice(record.signature().as_ref())
        .map_err(|_| invalid("malformed signature"))?;
    key.verify_strict(&action_bytes, &signature)
        .map_err(|_| invalid("signature verification failed"))?;
    Ok(Some(record))
}
