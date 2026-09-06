//! Authenticate immutable grant content and its exact Holochain authoring act.
//! No authority, issuer/recipient binding, freshness, or activation permission is implied.
use ed25519_dalek::VerifyingKey;
use holochain_types::prelude::*;

use super::conductor_writes::CreateMishpatCommitmentInput;
use crate::error::StorageError;

/// Bound decode, serialization, hashing and signature work before doing any of it.
pub const MAX_COMMITMENT_RECORD_BYTES: usize = 256 * 1024;
// mishpat/dna.yaml has one integrity zome; EntryTypes::Commitment is index 8.
const COMMITMENT_ZOME: u8 = 0;
const COMMITMENT_ENTRY: u8 = 8;

/// Independently provisioned pins; never infer these from the record being checked.
pub struct CommitmentRecordPins {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
    pub author: AgentPubKey,
}

/// An authenticated immutable record, not a delegation capability.
/// Private fields and no Deserialize implementation prevent bypassing verification.
pub struct AuthenticatedCommitmentRecord {
    pins: CommitmentRecordPins,
    content: CreateMishpatCommitmentInput,
}

impl AuthenticatedCommitmentRecord {
    pub fn action_hash(&self) -> &ActionHash {
        &self.pins.action_hash
    }
    pub fn entry_hash(&self) -> &EntryHash {
        &self.pins.entry_hash
    }
    pub fn author(&self) -> &AgentPubKey {
        &self.pins.author
    }
    pub fn content(&self) -> &CreateMishpatCommitmentInput {
        &self.content
    }
}

fn invalid(reason: &str) -> StorageError {
    StorageError::InvalidInput(format!("commitment record: {reason}"))
}

/// Decode the coordinator's Option<Record> and authenticate the exact pinned act.
/// Absence means only that this conductor could not return it, never revocation.
///
/// Precondition: the Record MUST have been obtained from the mishpat cell of the
/// acting node's own conductor; this function cannot detect a record from another
/// DNA whose zome 0 / entry 8 is also a public app entry.
pub(crate) fn verify_commitment_record(
    bytes: &[u8],
    pins: CommitmentRecordPins,
) -> Result<Option<AuthenticatedCommitmentRecord>, StorageError> {
    if bytes.len() > MAX_COMMITMENT_RECORD_BYTES {
        return Err(invalid("record exceeds byte limit"));
    }
    let record: Option<Record> =
        rmp_serde::from_slice(bytes).map_err(|_| invalid("malformed record"))?;
    let Some(record) = record else {
        return Ok(None);
    };
    let ActionData::Create(create) = &record.action().data else {
        return Err(invalid("expected Create"));
    };
    let EntryType::App(def) = &create.entry_type else {
        return Err(invalid("expected app entry"));
    };
    if def.zome_index != ZomeIndex(COMMITMENT_ZOME)
        || def.entry_index != EntryDefIndex(COMMITMENT_ENTRY)
        || def.visibility != EntryVisibility::Public
    {
        return Err(invalid("wrong Commitment entry definition"));
    }
    if record.action().author() != &pins.author {
        return Err(invalid("author pin mismatch"));
    }
    let Some(entry @ Entry::App(app)) = record.entry().as_option() else {
        return Err(invalid("missing app entry"));
    };
    if app.bytes().len() > MAX_COMMITMENT_RECORD_BYTES {
        return Err(invalid("entry exceeds byte limit"));
    }
    let action_hash = ActionHash::with_data_sync(record.action());
    if action_hash != pins.action_hash || record.action_address() != &action_hash {
        return Err(invalid("action pin mismatch"));
    }
    let entry_hash = EntryHash::with_data_sync(entry);
    if entry_hash != pins.entry_hash || create.entry_hash != entry_hash {
        return Err(invalid("entry pin mismatch"));
    }
    // Holochain signs serialized Action bytes, not its digest or JSON.
    let action_bytes = holochain_types::prelude::encode(record.action())
        .map_err(|_| invalid("action encoding failed"))?;
    let key_bytes: [u8; 32] = pins
        .author
        .get_raw_32()
        .try_into()
        .map_err(|_| invalid("malformed author key"))?;
    let key = VerifyingKey::from_bytes(&key_bytes).map_err(|_| invalid("invalid author key"))?;
    let signature = ed25519_dalek::Signature::from_slice(record.signature().as_ref())
        .map_err(|_| invalid("malformed signature"))?;
    key.verify_strict(&action_bytes, &signature)
        .map_err(|_| invalid("signature verification failed"))?;
    let content =
        rmp_serde::from_slice(app.bytes()).map_err(|_| invalid("malformed Commitment content"))?;
    Ok(Some(AuthenticatedCommitmentRecord { pins, content }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    // Independent fixture shape transcribed from mishpat_integrity::Commitment.
    #[derive(Debug, serde::Serialize)]
    struct Grant<'a> {
        action: &'a str,
        payload_json: &'a str,
        signed_at: &'a str,
    }

    async fn fixture(seed: u8) -> (Record, CommitmentRecordPins) {
        let key = SigningKey::from_bytes(&[seed; 32]);
        let author = AgentPubKey::from_raw_32(key.verifying_key().as_bytes().to_vec());
        let grant = Grant {
            action: "delegates-compute",
            payload_json: r#"{"scope":"runtime-upgrade"}"#,
            signed_at: "2026-09-06T00:00:00Z",
        };
        let entry = Entry::App(
            AppEntryBytes::try_from(SerializedBytes::from(UnsafeBytes::from(
                rmp_serde::to_vec_named(&grant).unwrap(),
            )))
            .unwrap(),
        );
        let entry_hash = EntryHash::with_data_sync(&entry);
        let action = Action {
            header: ActionHeader {
                author: author.clone(),
                timestamp: Timestamp::from_micros(123),
                action_seq: 4,
                prev_action: Some(ActionHash::from_raw_32(vec![9; 32])),
            },
            data: ActionData::Create(CreateData {
                entry_type: EntryType::App(AppEntryDef::new(
                    EntryDefIndex(8),
                    ZomeIndex(0),
                    EntryVisibility::Public,
                )),
                entry_hash: entry_hash.clone(),
            }),
        };
        // Use the SerializedBytes conversion used by Holochain's signing path,
        // independently of the verifier's encoder. Never sign a mocked digest.
        let signing_bytes = SerializedBytes::try_from(action.clone()).unwrap();
        let signature = Signature(key.sign(signing_bytes.bytes()).to_bytes());
        let signed = SignedActionHashed::new_unchecked(action, signature);
        let pins = CommitmentRecordPins {
            action_hash: signed.as_hash().clone(),
            entry_hash,
            author,
        };
        (Record::new(signed, RecordEntry::Present(entry)), pins)
    }
    fn wire(record: Record) -> Vec<u8> {
        rmp_serde::to_vec_named(&Some(record)).unwrap()
    }

    #[tokio::test]
    async fn exact_signed_record_authenticates_without_authority_or_effect() {
        let (record, pins) = fixture(1).await;
        let result = verify_commitment_record(&wire(record), pins)
            .unwrap()
            .unwrap();
        assert_eq!(result.content().action, "delegates-compute");
        assert_eq!(
            result.content().payload_json,
            r#"{"scope":"runtime-upgrade"}"#
        );
    }
    #[tokio::test]
    async fn identical_entry_from_another_author_is_not_the_pinned_act() {
        let (_, pins) = fixture(1).await;
        let (other, other_pins) = fixture(2).await;
        assert_eq!(pins.entry_hash, other_pins.entry_hash);
        assert_ne!(pins.action_hash, other_pins.action_hash);
        assert!(verify_commitment_record(&wire(other), pins).is_err());
    }
    #[tokio::test]
    async fn tampered_action_entry_signature_and_cached_hash_are_refused() {
        for mutation in 0..4 {
            let (mut record, pins) = fixture(1).await;
            match mutation {
                0 => record.signed_action.hashed.content.header.action_seq += 1,
                1 => {
                    record.entry = RecordEntry::Present(Entry::App(
                        AppEntryBytes::try_from(SerializedBytes::from(UnsafeBytes::from(vec![
                            0xc0,
                        ])))
                        .unwrap(),
                    ))
                }
                2 => record.signed_action.signature.0[0] ^= 1,
                _ => record.signed_action.hashed.hash = ActionHash::from_raw_32(vec![3; 32]),
            }
            assert!(
                verify_commitment_record(&wire(record), pins).is_err(),
                "mutation {mutation}"
            );
        }
    }
    #[tokio::test]
    async fn independent_pins_are_all_enforced() {
        for mutation in 0..3 {
            let (record, mut pins) = fixture(1).await;
            match mutation {
                0 => pins.action_hash = ActionHash::from_raw_32(vec![3; 32]),
                1 => pins.entry_hash = EntryHash::from_raw_32(vec![3; 32]),
                _ => {
                    pins.author = AgentPubKey::from_raw_32(
                        SigningKey::from_bytes(&[2; 32])
                            .verifying_key()
                            .as_bytes()
                            .to_vec(),
                    )
                }
            }
            assert!(verify_commitment_record(&wire(record), pins).is_err());
        }
    }
    #[tokio::test]
    async fn wrong_type_update_delete_and_missing_entry_are_refused() {
        for mutation in 0..6 {
            let (mut record, mut pins) = fixture(1).await;
            let action = &mut record.signed_action.hashed.content;
            match mutation {
                0 | 1 | 2 => {
                    if let ActionData::Create(c) = &mut action.data {
                        c.entry_type = EntryType::App(AppEntryDef::new(
                            EntryDefIndex(if mutation == 0 { 7 } else { 8 }),
                            ZomeIndex(if mutation == 1 { 1 } else { 0 }),
                            if mutation == 2 {
                                EntryVisibility::Private
                            } else {
                                EntryVisibility::Public
                            },
                        ));
                    }
                }
                3 => {
                    action.data = ActionData::Update(UpdateData {
                        original_action_address: pins.action_hash.clone(),
                        original_entry_address: pins.entry_hash.clone(),
                        entry_type: EntryType::App(AppEntryDef::new(
                            EntryDefIndex(8),
                            ZomeIndex(0),
                            EntryVisibility::Public,
                        )),
                        entry_hash: pins.entry_hash.clone(),
                    })
                }
                4 => {
                    action.data = ActionData::Delete(DeleteData {
                        deletes_address: pins.action_hash.clone(),
                        deletes_entry_address: pins.entry_hash.clone(),
                    })
                }
                _ => record.entry = RecordEntry::NotStored,
            }
            // Re-sign malformed shapes so refusal is about the shape, not stale hashes.
            let action = record.action().clone();
            let signing_bytes = SerializedBytes::try_from(action.clone()).unwrap();
            record.signed_action = SignedActionHashed::new_unchecked(
                action,
                Signature(
                    SigningKey::from_bytes(&[1; 32])
                        .sign(signing_bytes.bytes())
                        .to_bytes(),
                ),
            );
            pins.action_hash = record.action_address().clone();
            assert!(
                verify_commitment_record(&wire(record), pins).is_err(),
                "mutation {mutation}"
            );
        }
    }
    #[tokio::test]
    async fn malformed_oversized_and_absent_inputs_have_distinct_results() {
        for bytes in [vec![], vec![0xc1], vec![0; MAX_COMMITMENT_RECORD_BYTES + 1]] {
            let (_, pins) = fixture(1).await;
            assert!(verify_commitment_record(&bytes, pins).is_err());
        }
        let (_, pins) = fixture(1).await;
        assert!(verify_commitment_record(
            &rmp_serde::to_vec_named(&Option::<Record>::None).unwrap(),
            pins
        )
        .unwrap()
        .is_none());
    }
    #[tokio::test]
    async fn correctly_signed_malformed_commitment_content_is_refused() {
        let (mut record, mut pins) = fixture(1).await;
        let entry = Entry::App(
            AppEntryBytes::try_from(SerializedBytes::from(UnsafeBytes::from(vec![0xc0]))).unwrap(),
        );
        pins.entry_hash = EntryHash::with_data_sync(&entry);
        let mut action = record.action().clone();
        if let ActionData::Create(c) = &mut action.data {
            c.entry_hash = pins.entry_hash.clone();
        }
        let signing_bytes = SerializedBytes::try_from(action.clone()).unwrap();
        record.signed_action = SignedActionHashed::new_unchecked(
            action,
            Signature(
                SigningKey::from_bytes(&[1; 32])
                    .sign(signing_bytes.bytes())
                    .to_bytes(),
            ),
        );
        pins.action_hash = record.action_address().clone();
        record.entry = RecordEntry::Present(entry);
        assert!(verify_commitment_record(&wire(record), pins).is_err());
    }
}

#[cfg(test)]
mod schema_pin_tests {
    #[test]
    fn commitment_definition_matches_independent_integrity_source_and_manifest() {
        let source =
            include_str!("../../../holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs");
        let body = source
            .split("pub enum EntryTypes {")
            .nth(1)
            .unwrap()
            .split('}')
            .next()
            .unwrap();
        let variants: Vec<_> = body
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with("//") && !line.starts_with("#["))
            .collect();
        assert!(
            variants[usize::from(super::COMMITMENT_ENTRY)].starts_with("Commitment(Commitment)")
        );
        let manifest = include_str!("../../../holochain/dna/mishpat/dna.yaml");
        let integrity = manifest
            .split("integrity:")
            .nth(1)
            .unwrap()
            .split("coordinator:")
            .next()
            .unwrap();
        assert_eq!(integrity.matches("- name:").count(), 1);
        assert!(integrity.contains("- name: mishpat_integrity"));
        assert_eq!(super::COMMITMENT_ZOME, 0);
    }
}
