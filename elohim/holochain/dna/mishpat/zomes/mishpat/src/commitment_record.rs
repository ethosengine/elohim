//! Exact authoring-act readback. This proves neither current delegation nor non-revocation.
use hdk::prelude::*;
use mishpat_integrity::UnitEntryTypes;

/// Resolve one exact action, preserving its signature and entry bytes.
#[hdk_extern]
pub fn get_commitment_record(action_hash: ActionHash) -> ExternResult<Option<Record>> {
    let Some(record) = get(action_hash.clone(), GetOptions::default())? else {
        return Ok(None);
    };
    let expected: ScopedEntryDefIndex = UnitEntryTypes::Commitment.try_into()?;
    if record.action_address() != &action_hash
        || !is_commitment_create(&record, expected.zome_index, expected.zome_type)
    {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "expected exact Commitment Create record".into()
        )));
    }
    let entry = record.entry().as_option().expect("shape checked above");
    if entry
        .as_app_entry()
        .expect("shape checked above")
        .bytes()
        .len()
        > 256 * 1024
    {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Commitment entry exceeds byte limit".into()
        )));
    }
    if hash_action(record.action().clone())? != action_hash
        || record.action().entry_hash() != Some(&hash_entry(entry.clone())?)
    {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Commitment identity mismatch".into()
        )));
    }
    Ok(Some(record))
}

fn is_commitment_create(record: &Record, zome: ZomeIndex, index: EntryDefIndex) -> bool {
    matches!(&record.action().data, ActionData::Create(CreateData {
        entry_type: EntryType::App(def), ..
    }) if def.zome_index == zome && def.entry_index == index
        && def.visibility == EntryVisibility::Public)
        && matches!(record.entry().as_option(), Some(Entry::App(_)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(data: ActionData, entry: RecordEntry<Entry>) -> Record {
        let action = Action {
            header: ActionHeader {
                author: AgentPubKey::from_raw_32(vec![1; 32]),
                timestamp: Timestamp::from_micros(123),
                action_seq: 4,
                prev_action: Some(ActionHash::from_raw_32(vec![2; 32])),
            },
            data,
        };
        Record::new(
            SignedActionHashed::with_presigned(
                ActionHashed::with_pre_hashed(action, ActionHash::from_raw_32(vec![3; 32])),
                Signature([0; 64]),
            ),
            entry,
        )
    }

    #[test]
    fn exact_read_requires_commitment_create_type_and_present_entry() {
        for (zome, index, visibility, present, expected) in [
            (0, 8, EntryVisibility::Public, true, true),
            (1, 8, EntryVisibility::Public, true, false),
            (0, 7, EntryVisibility::Public, true, false),
            (0, 8, EntryVisibility::Private, true, false),
            (0, 8, EntryVisibility::Public, false, false),
        ] {
            let data = ActionData::Create(CreateData {
                entry_type: EntryType::App(AppEntryDef::new(
                    EntryDefIndex(index),
                    ZomeIndex(zome),
                    visibility,
                )),
                entry_hash: EntryHash::from_raw_32(vec![4; 32]),
            });
            let entry = if present {
                RecordEntry::Present(Entry::App(
                    AppEntryBytes::try_from(SerializedBytes::from(UnsafeBytes::from(vec![0xc0])))
                        .unwrap(),
                ))
            } else {
                RecordEntry::NotStored
            };
            assert_eq!(
                is_commitment_create(&fixture(data, entry), ZomeIndex(0), EntryDefIndex(8)),
                expected
            );
        }
        let delete = ActionData::Delete(DeleteData {
            deletes_address: ActionHash::from_raw_32(vec![3; 32]),
            deletes_entry_address: EntryHash::from_raw_32(vec![4; 32]),
        });
        assert!(!is_commitment_create(
            &fixture(delete, RecordEntry::NotStored),
            ZomeIndex(0),
            EntryDefIndex(8)
        ));
    }
}
