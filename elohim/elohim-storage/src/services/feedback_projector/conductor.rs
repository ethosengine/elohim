//! Production adapter for the content cell. Wire hashes stay typed until decoded.
use holochain_types::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::*;
use crate::conductor_admission::AdmissionClass;
use crate::conductor_bridge::{ConductorBridge, StaticConductorBridge};
use crate::hc_client::HcClient;

/// Reads the content cell through a [`ConductorBridge`], re-read on every
/// call — never a captured `Arc<HcClient>`. The feedback projector is a
/// long-lived durable-correction sweep (spawned once at boot); a conductor
/// pod roll used to leave it calling a corpse for the life of the process
/// (the defect class cured for the release-adoption controller — see
/// `crate::conductor_bridge` and
/// `genesis/data/timeline/backlog/self-heal-release-controller-dead-after-conductor-roll.md`).
pub struct ConductorReader {
    bridge: Arc<dyn ConductorBridge>,
    /// The origin DNA hash, memoized on first successful resolve. It is a
    /// property of the DNA the cell belongs to, not of any one connection —
    /// unchanged by a conductor restart — so caching it is correct, and
    /// necessary: `origin_dna_hash()` is sync and used as an EQUALITY key
    /// (`verify_record`'s foreign-DNA check), so a `""` fallback during a
    /// mid-remint window would misclassify every envelope as foreign rather
    /// than honestly pending.
    dna_hash: std::sync::OnceLock<String>,
}

fn invalid(e: impl std::fmt::Display) -> StorageError {
    StorageError::InvalidInput(format!("feedback record: {e}"))
}

impl ConductorReader {
    /// Construct with one fixed conductor handle. Correct only where no
    /// conductor restart can intervene between construction and use; kept for
    /// callers (and tests) that hold a client directly. Production wires
    /// [`Self::from_bridge`].
    pub fn new(hc: Arc<HcClient>) -> Self {
        Self::from_bridge(Arc::new(StaticConductorBridge(hc)))
    }

    /// The conductor leg as a LIVE source — production's wiring.
    pub fn from_bridge(bridge: Arc<dyn ConductorBridge>) -> Self {
        Self {
            bridge,
            dna_hash: std::sync::OnceLock::new(),
        }
    }

    async fn call<I: Serialize, O: DeserializeOwned>(
        &self,
        name: &str,
        input: I,
    ) -> Result<O, StorageError> {
        let payload = rmp_serde::to_vec_named(&input).map_err(invalid)?;
        // `Interactive` verbatim: this is the class `HcClient::call_zome`
        // applied before the bridge move ([`ConductorBridge::call_zome`]'s
        // convenience default is `Background`, which would silently re-class
        // every feedback read).
        let bytes = self
            .bridge
            .call_zome_classed("content_store", name, payload, AdmissionClass::Interactive)
            .await?;
        if bytes.len() > MAX_BYTES_PER_SWEEP {
            return Err(invalid("response exceeds sweep byte budget"));
        }
        rmp_serde::from_slice(&bytes).map_err(invalid)
    }
}

#[derive(Deserialize)]
#[cfg_attr(test, derive(Serialize))]
struct Refs {
    refs: Vec<Reference>,
}
#[derive(Deserialize)]
#[cfg_attr(test, derive(Serialize))]
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
        if let Some(cached) = self.dna_hash.get() {
            return cached.clone();
        }
        // The DNA hash is a property of the DNA, not of this connection — it
        // does not change across a conductor restart — so resolving it once
        // and memoizing is correct, and necessary: this is the sync method a
        // `None` handle (mid-remint) has no honest non-empty answer for.
        let Some(hc) = self.bridge.handle() else {
            return String::new();
        };
        let hash = hc.cell_id().dna_hash().to_string();
        let _ = self.dna_hash.set(hash.clone());
        hash
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    /// **The conductor rolled between two sweeps.** `ConductorReader` used to
    /// hold `Arc<HcClient>` directly, captured once at spawn time by the
    /// feedback-projector's boot wiring — the same defect class as the
    /// release-adoption controller and the soak-attestation rail. This bridge
    /// fake is dead until "the supervisor re-mints it" (`alive` flips), and
    /// asserts every read stays `Interactive`-classed (the class
    /// `HcClient::call_zome` applied before the bridge move).
    struct RollingBridge {
        alive: AtomicBool,
        handle_reads: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl ConductorBridge for RollingBridge {
        fn handle(&self) -> Option<Arc<HcClient>> {
            // A fake holds no real client; `call_zome_classed` stands in for
            // the whole leg and counts the re-read this trait exists for.
            None
        }

        async fn call_zome_classed(
            &self,
            _zome: &str,
            _fn_name: &str,
            _payload: Vec<u8>,
            class: AdmissionClass,
        ) -> Result<Vec<u8>, StorageError> {
            self.handle_reads.fetch_add(1, Ordering::SeqCst);
            assert!(
                matches!(class, AdmissionClass::Interactive),
                "feedback reads keep the Interactive class they had before the bridge move"
            );
            if !self.alive.load(Ordering::SeqCst) {
                return Err(StorageError::Connection(
                    "Websocket closed: No connection".to_string(),
                ));
            }
            let refs = Refs {
                refs: vec![Reference {
                    action_hash: ActionHash::from_raw_36(vec![0xAA; 36]),
                    fetch_outcome: "found".to_string(),
                }],
            };
            Ok(rmp_serde::to_vec_named(&refs).unwrap())
        }
    }

    #[tokio::test]
    async fn refs_for_target_lands_after_the_conductor_rolled_between_two_sweeps() {
        let bridge = Arc::new(RollingBridge {
            alive: AtomicBool::new(false),
            handle_reads: AtomicUsize::new(0),
        });
        // Built ONCE — as the feedback-projector's boot wiring builds it.
        let reader = ConductorReader::from_bridge(bridge.clone() as Arc<dyn ConductorBridge>);
        let target = ActionHash::from_raw_36(vec![0x11; 36]).to_string();

        // During the roll: refused honestly, nothing invented.
        let refused = reader
            .refs_for_target(&target)
            .await
            .expect_err("a rolled conductor cannot be read through");
        assert!(
            matches!(refused, StorageError::Connection(_)),
            "got {refused:?}"
        );

        // The supervisor re-minted the bridge. The SAME reader must follow it.
        bridge.alive.store(true, Ordering::SeqCst);
        let refs = reader
            .refs_for_target(&target)
            .await
            .expect("the re-minted conductor answered");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].fetch_outcome, "found");
        assert_eq!(
            bridge.handle_reads.load(Ordering::SeqCst),
            2,
            "the feedback reader re-reads its conductor leg per call; a handle captured at \
             spawn time is asked once and is a corpse from the first roll onward"
        );
    }

    /// `origin_dna_hash` cannot be resolved with no live handle EVER — the
    /// honest `""`, never a guess that would misclassify every envelope as
    /// foreign-DNA (see the field doc). Once resolved it survives a roll
    /// (memoized): the DNA hash does not change when the conductor restarts.
    #[test]
    fn origin_dna_hash_is_empty_with_no_bridge_ever_live_never_a_guess() {
        struct NeverAlive;
        #[async_trait::async_trait]
        impl ConductorBridge for NeverAlive {
            fn handle(&self) -> Option<Arc<HcClient>> {
                None
            }
        }
        let reader = ConductorReader::from_bridge(Arc::new(NeverAlive) as Arc<dyn ConductorBridge>);
        assert_eq!(reader.origin_dna_hash(), "");
        // Calling again must not panic or diverge — still honestly empty.
        assert_eq!(reader.origin_dna_hash(), "");
    }
}
