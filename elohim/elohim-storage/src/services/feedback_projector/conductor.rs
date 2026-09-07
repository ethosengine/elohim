//! Production adapter for the content cell. Wire hashes stay typed until decoded.
use holochain_types::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::*;
use crate::hc_client::HcClient;

pub struct ConductorReader(pub Arc<HcClient>);

fn invalid(e: impl std::fmt::Display) -> StorageError {
    StorageError::InvalidInput(format!("feedback record: {e}"))
}

impl ConductorReader {
    async fn call<I: Serialize, O: DeserializeOwned>(
        &self,
        name: &str,
        input: I,
    ) -> Result<O, StorageError> {
        let payload = rmp_serde::to_vec_named(&input).map_err(invalid)?;
        let bytes = self.0.call_zome("content_store", name, payload).await?;
        if bytes.len() > MAX_BYTES_PER_SWEEP {
            return Err(invalid("response exceeds sweep byte budget"));
        }
        rmp_serde::from_slice(&bytes).map_err(invalid)
    }
}

#[derive(Deserialize)]
struct Refs {
    refs: Vec<Reference>,
}
#[derive(Deserialize)]
struct Reference {
    action_hash: ActionHash,
    fetch_outcome: String,
}
#[derive(Deserialize)]
struct Lineage {
    referenced_action_hash: ActionHash,
    root_action_hash: ActionHash,
    root_author: AgentPubKey,
    content_id: String,
    candidates: Vec<Candidate>,
    head_action_hash: Option<ActionHash>,
    contested: bool,
    contested_predecessors: Vec<ActionHash>,
}
#[derive(Deserialize)]
struct Candidate {
    action_hash: ActionHash,
    predecessor: Option<ActionHash>,
    author: Option<AgentPubKey>,
    timestamp: Option<Timestamp>,
    fetch_outcome: String,
    in_root: bool,
}

#[async_trait::async_trait]
impl FeedbackDhtReader for ConductorReader {
    fn origin_dna_hash(&self) -> String {
        self.0.cell_id().dna_hash().to_string()
    }

    async fn refs_for_target(&self, target: &str) -> Result<Vec<DiscoveredRef>, StorageError> {
        #[derive(Serialize)]
        struct Input {
            target_action_hash: ActionHash,
            resolve: bool,
        }
        let refs: Refs = self
            .call(
                "get_feedback_signal_refs_for_target",
                Input {
                    target_action_hash: parse_action(target)?,
                    resolve: false,
                },
            )
            .await?;
        Ok(refs
            .refs
            .into_iter()
            .map(|r| DiscoveredRef {
                action_hash: r.action_hash.to_string(),
                fetch_outcome: r.fetch_outcome,
            })
            .collect())
    }

    async fn signal_record(&self, hash: &str) -> Result<Option<FetchedRecord>, StorageError> {
        let requested = parse_action(hash)?;
        let record: Option<Record> = self
            .call("get_feedback_signal_record", requested.clone())
            .await?;
        let Some(record) = record else {
            return Ok(None);
        };
        let fetched = decode_record(record, &requested)?;
        if let Some(entry) = fetched
            .entry
            .as_ref()
            .filter(|e| e.signal_kind == "correction")
        {
            let Some(evidence) = entry.evidence_cid.as_deref() else {
                return Err(invalid("correction has no evidence"));
            };
            let content: Option<lamad_types::ContentOutput> =
                self.call("get_content", parse_action(evidence)?).await?;
            let Some(content) = content else {
                return Ok(None);
            };
            let metadata: serde_json::Value =
                serde_json::from_str(&content.content.metadata_json).map_err(invalid)?;
            let request: crate::api::feedback_operations::CorrectionRequest =
                serde_json::from_value(
                    metadata
                        .get("correctionRequest")
                        .cloned()
                        .ok_or_else(|| invalid("missing correctionRequest"))?,
                )
                .map_err(invalid)?;
            if request.operation_id.trim().is_empty()
                || request.target_action_hash != entry.target_cid
                || request.signal_kind != entry.signal_kind
                || request.standing_impact != entry.standing_impact
                || !["public", "commons"].contains(&content.content.reach.as_str())
            {
                return Err(invalid("correction evidence request mismatch"));
            }
        }
        Ok(Some(fetched))
    }

    async fn content_lineage(&self, hash: &str) -> Result<Option<ContentLineage>, StorageError> {
        #[derive(Serialize)]
        struct Input {
            action_hash: ActionHash,
            local: bool,
        }
        let lineage: Lineage = self
            .call(
                "get_content_lineage",
                Input {
                    action_hash: parse_action(hash)?,
                    local: true,
                },
            )
            .await?;
        let l = lineage;
        Ok(Some(ContentLineage {
            referenced_action_hash: l.referenced_action_hash.to_string(),
            root_action_hash: l.root_action_hash.to_string(),
            root_author: l.root_author.to_string(),
            content_id: l.content_id,
            head_action_hash: l.head_action_hash.map(|h| h.to_string()),
            contested: l.contested,
            contested_predecessors: l
                .contested_predecessors
                .into_iter()
                .map(|h| h.to_string())
                .collect(),
            candidates: l
                .candidates
                .into_iter()
                .map(|c| LineageCandidate {
                    action_hash: c.action_hash.to_string(),
                    predecessor: c.predecessor.map(|h| h.to_string()),
                    author: c.author.map(|h| h.to_string()),
                    timestamp: c.timestamp.map(|t| t.as_micros()),
                    fetch_outcome: c.fetch_outcome,
                    in_root: c.in_root,
                })
                .collect(),
        }))
    }
}

fn decode_record(record: Record, requested: &ActionHash) -> Result<FetchedRecord, StorageError> {
    let action = record.action();
    let ActionData::Create(create) = &action.data else {
        return Err(invalid("expected immutable Create"));
    };
    let EntryType::App(def) = &create.entry_type else {
        return Err(invalid("expected App entry"));
    };
    // content_store_integrity is the sole integrity zome; FeedbackSignal is entry 13.
    if def.zome_index != ZomeIndex(0)
        || def.entry_index != EntryDefIndex(13)
        || def.visibility != EntryVisibility::Public
    {
        return Err(invalid("wrong FeedbackSignal entry definition"));
    }
    let computed = ActionHash::with_data_sync(action);
    if &computed != requested || record.action_address() != &computed {
        return Err(invalid("action hash mismatch"));
    }
    let key = ed25519_dalek::VerifyingKey::from_bytes(
        action.author().get_raw_32().try_into().map_err(invalid)?,
    )
    .map_err(invalid)?;
    let signature =
        ed25519_dalek::Signature::from_slice(record.signature().as_ref()).map_err(invalid)?;
    key.verify_strict(
        &holochain_types::prelude::encode(action).map_err(invalid)?,
        &signature,
    )
    .map_err(invalid)?;
    let (bound, entry, len) = match record.entry().as_option() {
        Some(entry @ Entry::App(app)) => {
            if app.bytes().len() > MAX_BYTES_PER_SWEEP {
                return Err(invalid("entry exceeds sweep byte budget"));
            }
            (
                EntryHash::with_data_sync(entry) == create.entry_hash,
                Some(rmp_serde::from_slice(app.bytes()).map_err(invalid)?),
                app.bytes().len(),
            )
        }
        _ => (false, None, 0),
    };
    Ok(FetchedRecord {
        action_hash: computed.to_string(),
        author_raw: action.author().get_raw_39().to_vec(),
        timestamp_micros: action.timestamp().as_micros(),
        entry_hash_bound: bound,
        entry,
        entry_bytes_len: len,
    })
}

fn parse_action(value: &str) -> Result<ActionHash, StorageError> {
    let encoded = value
        .strip_prefix('u')
        .ok_or_else(|| invalid("missing hash prefix"))?;
    let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(invalid)?;
    ActionHash::try_from_raw_39(raw).map_err(invalid)
}
