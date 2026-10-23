//! Witness facts for one record — what this conductor can say about whether the
//! network has witnessed an action.
//!
//! The content chrome shows `green` once a conductor OTHER than the author holds
//! the record's action-basis op as valid, and `amber` until then. Storage
//! decides that reading (`elohim-storage::services::content_witness`); this
//! extern hands it the three local facts it decides from:
//!
//! - **author** — who signed the action, so storage can tell "authored on this
//!   conductor" from "came here from DHT authorities";
//! - **receipts** — the validation receipts other conductors sent back for the
//!   action's ops. Receipts are sent only to the authoring conductor, so a
//!   non-zero count also proves the action was authored here;
//! - **held_valid** — whether this conductor holds the record locally with a
//!   `Valid` validation status.
//!
//! Why receipts are counted and `receipts_complete` is NOT read: the pinned
//! conductor marks an op complete only at its entry def's `required_validations`
//! (5 unless declared; no entry type in this DNA declares one), and a receipt
//! comes from a remote conductor, one per conductor. A three-conductor
//! household can return at most two, so `receipts_complete` stays false there
//! however long the record has been witnessed (measured 2026-10-16: 0 of
//! 73,533 published ops complete on matthew; see the backlog entry
//! `genesis/data/timeline/backlog/fork-receipt-target-fixed-republish-cost.md`).
//!
//! Coordinator-only and read-only: one local receipt read and one local
//! `get_details`, no network await, no commit, no link — so it ships on the
//! `update_coordinators` hot-swap path and does not move the DNA hash.

use hdk::prelude::*;

/// The op types whose basis is the action itself — "the record's op". The
/// pinned fork names them `CreateRecord` / `UpdateRecord` / `DeleteRecord`;
/// stock Holochain 0.7 names the same op `StoreRecord`. `ValidationReceiptSet`
/// carries the name as an informational string, so both spellings are read.
pub const RECORD_OP_TYPES: [&str; 4] = [
    "StoreRecord",
    "CreateRecord",
    "UpdateRecord",
    "DeleteRecord",
];

/// What this conductor holds about one action. Never stored; recomputed on
/// every call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordWitnessFacts {
    /// The action's author (base64, `uhCAk…`).
    pub author: holo_hash::AgentPubKeyB64,
    /// Whether the author is the calling cell's own agent. Storage's other
    /// local agents (hosted cells on the same conductor) are its to recognise.
    pub authored_by_caller: bool,
    /// Whether this conductor holds the record with `ValidationStatus::Valid`.
    pub held_valid: bool,
    /// Receipts received for the record's op (see [`RECORD_OP_TYPES`]).
    pub record_op_receipts: u32,
    /// Of those, the receipts whose validator reported `Valid`.
    pub record_op_valid_receipts: u32,
    /// `Valid` receipts across every op of the action, the record op included.
    pub valid_receipts_all_ops: u32,
}

/// Receipt counts for one action, from its receipt sets:
/// `(record_op_receipts, record_op_valid_receipts, valid_receipts_all_ops)`.
pub fn count_receipts(sets: &[ValidationReceiptSet]) -> (u32, u32, u32) {
    let mut record_op = 0u32;
    let mut record_op_valid = 0u32;
    let mut valid_all = 0u32;
    for set in sets {
        let is_record_op = RECORD_OP_TYPES.contains(&set.op_type.as_str());
        for receipt in &set.receipts {
            let valid = receipt.validation_status == ValidationStatus::Valid;
            if valid {
                valid_all = valid_all.saturating_add(1);
            }
            if is_record_op {
                record_op = record_op.saturating_add(1);
                if valid {
                    record_op_valid = record_op_valid.saturating_add(1);
                }
            }
        }
    }
    (record_op, record_op_valid, valid_all)
}

/// The witness facts for `action_hash`, or `None` when this conductor holds no
/// record for it (nothing to say — storage reads that as amber).
#[hdk_extern]
pub fn get_record_witness_facts(
    action_hash: holo_hash::ActionHashB64,
) -> ExternResult<Option<RecordWitnessFacts>> {
    let action_hash: ActionHash = action_hash.into();
    let Some(Details::Record(details)) = get_details(action_hash.clone(), GetOptions::local())?
    else {
        return Ok(None);
    };
    let author = details.record.action().author().clone();
    let held_valid = details.validation_status == ValidationStatus::Valid;
    let sets = get_validation_receipts(GetValidationReceiptsInput::new(action_hash))?;
    let (record_op_receipts, record_op_valid_receipts, valid_receipts_all_ops) =
        count_receipts(&sets);
    let authored_by_caller = agent_info()?.agent_initial_pubkey == author;
    Ok(Some(RecordWitnessFacts {
        author: author.into(),
        authored_by_caller,
        held_valid,
        record_op_receipts,
        record_op_valid_receipts,
        valid_receipts_all_ops,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(op_type: &str, statuses: &[ValidationStatus]) -> ValidationReceiptSet {
        ValidationReceiptSet {
            op_hash: DhtOpHash::from_raw_36(vec![0xdb; 36]),
            op_type: op_type.to_string(),
            receipts_complete: false,
            receipts: statuses
                .iter()
                .map(|s| ValidationReceiptInfo {
                    validation_status: *s,
                    validators: vec![AgentPubKey::from_raw_36(vec![0xa1; 36])],
                })
                .collect(),
        }
    }

    #[test]
    fn counts_record_op_receipts_under_either_op_name() {
        use ValidationStatus::*;
        // The fork's spelling, as measured on the household (two remote
        // conductors, each one receipt per op).
        let fork = [
            set("CreateRecord", &[Valid, Valid]),
            set("CreateEntry", &[Valid, Valid]),
            set("AgentActivity", &[Valid]),
        ];
        assert_eq!(count_receipts(&fork), (2, 2, 5));
        // Stock 0.7's spelling counts the same.
        assert_eq!(count_receipts(&[set("StoreRecord", &[Valid])]), (1, 1, 1));
        assert_eq!(count_receipts(&[set("UpdateRecord", &[Valid])]), (1, 1, 1));
    }

    #[test]
    fn a_rejecting_receipt_is_counted_but_not_valid() {
        use ValidationStatus::*;
        assert_eq!(
            count_receipts(&[set("CreateRecord", &[Rejected, Abandoned])]),
            (2, 0, 0)
        );
    }

    #[test]
    fn receipts_on_other_ops_do_not_count_as_the_record_op() {
        use ValidationStatus::*;
        assert_eq!(count_receipts(&[set("CreateEntry", &[Valid])]), (0, 0, 1));
        assert_eq!(count_receipts(&[]), (0, 0, 0));
    }

    #[test]
    fn facts_round_trip_through_messagepack_with_the_author_as_a_string() {
        // A real hash (location bytes computed), so the b64 form round-trips.
        let author = AgentPubKey::from_raw_32(vec![0xa2; 32]);
        let facts = RecordWitnessFacts {
            author: author.clone().into(),
            authored_by_caller: true,
            held_valid: true,
            record_op_receipts: 2,
            record_op_valid_receipts: 2,
            valid_receipts_all_ops: 6,
        };
        let bytes = rmp_serde::to_vec_named(&facts).unwrap();
        // Storage decodes the author as a plain string — the b64 form, never
        // the raw hash bytes.
        #[derive(serde::Deserialize)]
        struct StorageSide {
            author: String,
            held_valid: bool,
            record_op_valid_receipts: u32,
        }
        let read: StorageSide = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(read.author, author.to_string());
        assert!(read.author.starts_with("uhCAk"));
        assert!(read.held_valid);
        assert_eq!(read.record_op_valid_receipts, 2);
        let back: RecordWitnessFacts = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(back, facts);
    }
}
