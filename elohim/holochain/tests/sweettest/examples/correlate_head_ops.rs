//! Read-only correlation of fork dump-full-state and dump-op-timings pages.
//! Usage: correlate_head_ops request.json > receiver-integration.json
//! Capture bounded pages on EACH receiver's own admin port, with the pinned fork:
//! `hc-client call --port PORT dump-full-state DNA AGENT --limit 256 > state-0.json`
//! `hc-client call --port PORT dump-op-timings DNA --limit 256 > timings-0.json`
//! Continue with `--cursor WHEN_RECEIVED HASH` from, respectively,
//! `integration_dump.dht_ops_cursor` and `cursor` until all necessary ops are present.
//! Both cursors expose `when_received` and `hash`; retain every captured page.
//! Request JSON fields: content_id, dna, head, root, election_link, optional issuance_action_hash and
//! acceptance_witness_hash (raw 39-byte arrays), deadline_micros,
//! state_pages, timing_pages. Use the native observer's EXACT hashes and shared deadlineMs * 1000;
//! page lists are local file paths. Missing history, cache-only ops, rejected ops,
//! and integration after that deadline all fail. Capture may occur later: recorded
//! integration clocks establish timing, not the time the dump was downloaded.
//! Storage/doorway first-observed clocks remain separate publisher observations.
//! No timestamps are invented: validation is a boolean native fact, integration
//! is the conductor's recorded clock. Capture each receiver independently.
use anyhow::{anyhow, ensure, Context, Result};
use holochain_types::op::{ChainOp, DhtOp, OpEntry};
use holochain_types::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Deserialize)]
struct Request {
    content_id: String,
    dna: DnaHash,
    head: ActionHash,
    root: ActionHash,
    election_link: ActionHash,
    #[serde(default)]
    issuance_action_hash: Option<ActionHash>,
    #[serde(default)]
    acceptance_witness_hash: Option<ActionHash>,
    deadline_micros: i64,
    state_pages: Vec<PathBuf>,
    timing_pages: Vec<PathBuf>,
}
#[derive(Deserialize)]
struct StatePage {
    integration_dump: Integration,
}
#[derive(Deserialize)]
struct Integration {
    integrated: Vec<DhtOp>,
}
#[derive(Deserialize)]
struct TimingPage {
    timings: Vec<Timing>,
}
#[derive(Deserialize, Serialize, Clone, Debug)]
struct Timing {
    op_hash: DhtOpHash,
    when_received: Timestamp,
    when_integrated: Option<Timestamp>,
    abandoned_at: Option<Timestamp>,
    validation_status: Option<OpValidity>,
    locally_validated: Option<bool>,
}
#[derive(Serialize)]
struct ProvenOp {
    action: ActionHash,
    kind: &'static str,
    timing: Timing,
}
#[derive(Serialize)]
struct Evidence {
    content_id: String,
    dna: DnaHash,
    head: ActionHash,
    root: ActionHash,
    election_link: ActionHash,
    deadline_micros: i64,
    history: Vec<ProvenOp>,
    election: ProvenOp,
    issuance: Option<ProvenOp>,
    acceptance: Option<ProvenOp>,
    acceptance_author_history: Vec<ProvenOp>,
}

#[derive(Deserialize, Serialize, Debug)]
struct DelegationEvidence {
    payload: DelegationPayloadEvidence,
    signature: Signature,
    acceptance: AcceptanceEvidence,
}
#[derive(Deserialize, Serialize, Debug)]
struct DelegationPayloadEvidence {
    grantor: AgentPubKey,
    delegate: AgentPubKey,
    scope: String,
    valid_until: Timestamp,
    root_action_hash: ActionHash,
    dna_hash: DnaHash,
    #[serde(default)]
    issuance_action_hash: Option<ActionHash>,
}
#[derive(Deserialize, Serialize, Debug)]
struct AcceptanceEvidence {
    head_action_hash: ActionHash,
    witness_action_hash: ActionHash,
    accepted_at: Timestamp,
}

/// Derive the requirement from the integrated election itself, never an optional
/// caller field which could omit a delegated publication's acceptance witness.
fn required_acceptance(tag: &[u8]) -> Result<Option<DelegationEvidence>> {
    if tag == b"canonical-head:earned" {
        return Ok(None);
    }
    let bytes = tag
        .strip_prefix(b"canonical-head:earned|delegation:")
        .context("election is not an exact earned declaration tag")?;
    Ok(Some(holochain_serialized_bytes::decode(&bytes).context(
        "delegated election lacks a decodable native acceptance witness",
    )?))
}

fn verify_witness_context(
    witness: &ChainOp,
    grant: &DelegationEvidence,
    root_hash: &ActionHash,
    head: &ActionHash,
    root_author: &AgentPubKey,
    content_id: &str,
    dna: &DnaHash,
    election_type: (ZomeIndex, LinkType),
) -> Result<()> {
    let action = witness.signed_action();
    ensure!(
        action.data().author() == root_author,
        "acceptance witness has another author"
    );
    ensure!(
        &grant.payload.grantor == root_author
            && grant.payload.scope == content_id
            && &grant.payload.root_action_hash == root_hash
            && &grant.payload.dna_hash == dna,
        "acceptance grant context differs"
    );
    ensure!(
        grant.acceptance.accepted_at < grant.payload.valid_until,
        "acceptance is not before grant expiry"
    );
    ensure!(
        action.data().timestamp() == grant.acceptance.accepted_at,
        "acceptance time differs from native action"
    );
    ensure!(
        &grant.acceptance.head_action_hash == head,
        "acceptance names another head"
    );
    let ActionData::CreateLink(link) = &action.data().data else {
        anyhow::bail!("acceptance witness is not CreateLink");
    };
    ensure!(
        link.base_address.clone().into_action_hash().as_ref() == Some(root_hash)
            && link.target_address.clone().into_action_hash().as_ref() == Some(head),
        "acceptance witness names another root/head"
    );
    ensure!(
        (link.zome_index, link.link_type) == election_type,
        "acceptance witness has another scoped link type"
    );
    let tag = [
        b"head-acceptance:v2:".as_slice(),
        grant.signature.as_ref(),
        head.get_raw_39(),
    ]
    .concat();
    ensure!(
        link.tag.0 == tag,
        "acceptance witness tag differs from exact grant/head"
    );
    Ok(())
}

fn verify_issuance_context(
    issuer: &ChainOp,
    grant: &DelegationEvidence,
    root_record: &ChainOp,
    root_hash: &ActionHash,
    root_author: &AgentPubKey,
    link_type: (ZomeIndex, LinkType),
) -> Result<()> {
    let expected_hash = grant
        .payload
        .issuance_action_hash
        .as_ref()
        .context("v3 grant has no issuance action hash")?;
    let action = issuer.signed_action().data();
    ensure!(
        &ActionHash::with_data_sync(action) == expected_hash,
        "grant issuance action hash differs"
    );
    ensure!(
        action.author() == root_author,
        "grant issuance has another author"
    );
    let root = root_record.signed_action().data();
    ensure!(
        &ActionHash::with_data_sync(root) == root_hash,
        "root record mismatch"
    );
    ensure!(
        root.action_seq() < action.action_seq() && root.timestamp() < action.timestamp(),
        "grant issuance does not follow immutable root"
    );
    ensure!(
        action.timestamp() < grant.payload.valid_until,
        "grant issuance is at or after expiry"
    );
    let ActionData::CreateLink(link) = &action.data else {
        anyhow::bail!("grant issuance is not CreateLink");
    };
    let expected_tag = [
        b"head-delegation-issued:v1:".as_slice(),
        &grant.payload.valid_until.as_micros().to_be_bytes(),
    ]
    .concat();
    ensure!(
        link.base_address.clone().into_action_hash().as_ref() == Some(root_hash)
            && link.target_address == AnyLinkableHash::from(grant.payload.delegate.clone())
            && (link.zome_index, link.link_type) == link_type
            && link.tag.0 == expected_tag,
        "grant issuance link context differs"
    );
    ensure!(
        &grant.payload.root_action_hash == root_hash,
        "grant issuance payload names another root"
    );
    Ok(())
}
fn read<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> Result<T> {
    serde_json::from_slice(&std::fs::read(path)?).with_context(|| path.display().to_string())
}
fn verified_timing(
    op: &ChainOp,
    timings: &HashMap<DhtOpHash, Timing>,
    deadline: i64,
) -> Result<Timing> {
    let hash = op.to_hash(); // Core API: never duplicate protocol hashing in JS.
    let t = timings
        .get(&hash)
        .ok_or_else(|| anyhow!("missing timing for {hash}"))?;
    ensure!(
        t.validation_status == Some(OpValidity::Accepted),
        "op {hash} is not accepted"
    );
    ensure!(
        t.locally_validated == Some(true),
        "op {hash} is cache-only or not locally validated"
    );
    ensure!(t.abandoned_at.is_none(), "op {hash} was abandoned");
    let integrated = t
        .when_integrated
        .ok_or_else(|| anyhow!("op {hash} is not integrated"))?;
    ensure!(
        integrated.as_micros() <= deadline,
        "op {hash} integrated after shared deadline"
    );
    Ok(t.clone())
}
fn correlate(
    request: Request,
    ops: Vec<DhtOp>,
    timings: HashMap<DhtOpHash, Timing>,
) -> Result<Evidence> {
    let mut records = HashMap::new();
    let mut author_records = HashMap::new();
    let mut links = HashMap::new();
    for op in ops {
        if let DhtOp::ChainOp(op) = op {
            let action = ActionHash::with_data_sync(op.signed_action().data());
            // Every signed source-chain action can be a predecessor, including
            // CreateLink issue and revoke actions.
            author_records.insert(action.clone(), op.clone());
            match op.as_ref() {
                ChainOp::CreateRecord(_, OpEntry::Present(_)) => {
                    records.insert(action, op);
                }
                ChainOp::CreateLink(_) => {
                    links.insert(action, op);
                }
                _ => {}
            }
        }
    }
    let mut cursor = request.head.clone();
    let mut seen = std::collections::HashSet::new();
    let mut history = Vec::new();
    loop {
        ensure!(
            seen.insert(cursor.clone()) && seen.len() <= 256,
            "cyclic or overlong history"
        );
        let op = records
            .get(&cursor)
            .ok_or_else(|| anyhow!("missing integrated entry-bearing record {cursor}"))?;
        let timing = verified_timing(op, &timings, request.deadline_micros)?;
        history.push(ProvenOp {
            action: cursor.clone(),
            kind: "CreateRecord",
            timing,
        });
        match &op.signed_action().data().data {
            ActionData::Update(update) => cursor = update.original_action_address.clone(),
            ActionData::Create(_) => {
                ensure!(
                    cursor == request.root,
                    "history ends at another immutable root"
                );
                break;
            }
            _ => anyhow::bail!("content history contains a non-content action"),
        }
    }
    let link = links
        .get(&request.election_link)
        .ok_or_else(|| anyhow!("election CreateLink not integrated"))?;
    let ActionData::CreateLink(data) = &link.signed_action().data().data else {
        anyhow::bail!("election is not CreateLink")
    };
    ensure!(
        data.target_address.clone().into_action_hash().as_ref() == Some(&request.head),
        "election names another head"
    );
    let election = ProvenOp {
        action: request.election_link.clone(),
        kind: "CreateLink",
        timing: verified_timing(link, &timings, request.deadline_micros)?,
    };
    let mut acceptance_author_history = Vec::new();
    let mut issuance = None;
    let acceptance = if let Some(grant) = required_acceptance(&data.tag.0)? {
        ensure!(
            request.acceptance_witness_hash.as_ref() == Some(&grant.acceptance.witness_action_hash),
            "capture request witness differs from integrated election grant"
        );
        ensure!(
            request.issuance_action_hash == grant.payload.issuance_action_hash,
            "capture request issuance differs from integrated election grant"
        );
        let witness = links
            .get(&grant.acceptance.witness_action_hash)
            .ok_or_else(|| anyhow!("acceptance CreateLink not integrated"))?;
        let root_record = records
            .get(&request.root)
            .ok_or_else(|| anyhow!("root record missing"))?;
        verify_witness_context(
            witness,
            &grant,
            &request.root,
            &request.head,
            root_record.signed_action().data().author(),
            &request.content_id,
            &request.dna,
            (data.zome_index, data.link_type),
        )?;
        let history_anchor = if let Some(issuance_hash) = &grant.payload.issuance_action_hash {
            let issuer = links
                .get(issuance_hash)
                .ok_or_else(|| anyhow!("grant issuance CreateLink not integrated"))?;
            verify_issuance_context(
                issuer,
                &grant,
                root_record,
                &request.root,
                root_record.signed_action().data().author(),
                (data.zome_index, data.link_type),
            )?;
            issuance = Some(ProvenOp {
                action: issuance_hash.clone(),
                kind: "CreateLink:grant-issuance",
                timing: verified_timing(issuer, &timings, request.deadline_micros)?,
            });
            issuance_hash.clone()
        } else {
            request.root.clone()
        };
        let mut current = witness.signed_action().data().clone();
        let revocation_tag = match &grant.payload.issuance_action_hash {
            Some(issuance_hash) => [
                b"head-delegation-revoked:v2:".as_slice(),
                issuance_hash.get_raw_39(),
            ]
            .concat(),
            None => [
                b"head-delegation-revoked:v1:".as_slice(),
                grant.signature.as_ref(),
            ]
            .concat(),
        };
        // Match native 4096 total records, including the witness itself.
        for _ in 1..4096 {
            let previous = current
                .prev_action()
                .context("acceptance author history has no grant anchor")?;
            let prior = author_records
                .get(previous)
                .context("acceptance author history action not integrated")?;
            let action = prior.signed_action();
            ensure!(
                action.data().author() == root_record.signed_action().data().author()
                    && action.data().action_seq().checked_add(1) == Some(current.action_seq())
                    && action.data().timestamp() < current.timestamp(),
                "acceptance author history sequence differs"
            );
            ensure!(
                !matches!(&action.data().data, ActionData::CreateLink(link)
                if link.base_address.clone().into_action_hash().as_ref() == Some(&request.root)
                    && link.target_address == AnyLinkableHash::from(grant.payload.delegate.clone())
                    && link.zome_index == data.zome_index && link.link_type == data.link_type
                    && link.tag.0 == revocation_tag),
                "acceptance follows revocation"
            );
            acceptance_author_history.push(ProvenOp {
                action: previous.clone(),
                kind: if previous == &history_anchor {
                    "grant-anchor:issuance-or-root"
                } else {
                    "acceptance-author-history"
                },
                timing: verified_timing(prior, &timings, request.deadline_micros)?,
            });
            if previous == &history_anchor {
                break;
            }
            current = action.data().clone();
        }
        ensure!(
            acceptance_author_history
                .last()
                .is_some_and(|op| op.action == history_anchor),
            "acceptance author history budget exceeded"
        );
        Some(ProvenOp {
            action: grant.acceptance.witness_action_hash,
            kind: "CreateLink:acceptance",
            timing: verified_timing(witness, &timings, request.deadline_micros)?,
        })
    } else {
        ensure!(
            request.issuance_action_hash.is_none() && request.acceptance_witness_hash.is_none(),
            "capture request carries grant actions for an un-delegated election"
        );
        None
    };
    Ok(Evidence {
        content_id: request.content_id,
        dna: request.dna,
        head: request.head,
        root: request.root,
        election_link: request.election_link,
        deadline_micros: request.deadline_micros,
        history,
        election,
        issuance,
        acceptance,
        acceptance_author_history,
    })
}
fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .context("usage: correlate_head_ops request.json")?;
    let request: Request = read(std::path::Path::new(&path))?;
    let mut ops = Vec::new();
    let mut timings = HashMap::new();
    for path in &request.state_pages {
        ops.extend(read::<StatePage>(path)?.integration_dump.integrated);
    }
    for path in &request.timing_pages {
        for timing in read::<TimingPage>(path)?.timings {
            timings.insert(timing.op_hash.clone(), timing);
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&correlate(request, ops, timings)?)?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (ChainOp, HashMap<DhtOpHash, Timing>) {
        let action = Action {
            header: ActionHeader {
                author: AgentPubKey::from_raw_32(vec![1; 32]),
                timestamp: Timestamp::from_micros(1),
                action_seq: 4,
                prev_action: Some(ActionHash::from_raw_32(vec![2; 32])),
            },
            data: ActionData::Create(CreateData {
                entry_type: EntryType::App(AppEntryDef::new(
                    0.into(),
                    0.into(),
                    EntryVisibility::Public,
                )),
                entry_hash: EntryHash::from_raw_32(vec![3; 32]),
            }),
        };
        let op = ChainOp::CreateRecord(
            SignedAction::new(action, Signature([0; 64])),
            OpEntry::ActionOnly,
        );
        let timing = Timing {
            op_hash: op.to_hash(),
            when_received: Timestamp::from_micros(10),
            when_integrated: Some(Timestamp::from_micros(20)),
            abandoned_at: None,
            validation_status: Some(OpValidity::Accepted),
            locally_validated: Some(true),
        };
        (op, HashMap::from([(timing.op_hash.clone(), timing)]))
    }
    #[test]
    fn cache_only_and_late_integration_are_not_native_acceptance() {
        let (op, mut timings) = fixture();
        assert!(verified_timing(&op, &timings, 20).is_ok());
        assert!(verified_timing(&op, &timings, 19).is_err());
        timings.get_mut(&op.to_hash()).unwrap().locally_validated = Some(false);
        assert!(verified_timing(&op, &timings, 20)
            .unwrap_err()
            .to_string()
            .contains("cache-only"));
    }
    #[test]
    fn missing_or_rejected_validation_never_passes() {
        let (op, mut timings) = fixture();
        timings.get_mut(&op.to_hash()).unwrap().validation_status = None;
        assert!(verified_timing(&op, &timings, 20).is_err());
        timings.clear();
        assert!(verified_timing(&op, &timings, 20).is_err());
    }
    #[test]
    fn v3_capture_anchors_at_integrated_issuance_after_ancient_root() {
        let author = AgentPubKey::from_raw_32(vec![1; 32]);
        let delegate = AgentPubKey::from_raw_32(vec![2; 32]);
        let dna = DnaHash::from_raw_32(vec![4; 32]);
        let root_action = Action {
            header: ActionHeader {
                author: author.clone(),
                timestamp: Timestamp::from_micros(1),
                action_seq: 319,
                prev_action: Some(ActionHash::from_raw_32(vec![3; 32])),
            },
            data: ActionData::Create(CreateData {
                entry_type: EntryType::App(AppEntryDef::new(
                    0.into(),
                    0.into(),
                    EntryVisibility::Public,
                )),
                entry_hash: EntryHash::from_raw_32(vec![5; 32]),
            }),
        };
        let root = ActionHash::with_data_sync(&root_action);
        let root_op = ChainOp::CreateRecord(
            SignedAction::new(root_action.clone(), Signature([0; 64])),
            OpEntry::Present(Entry::Agent(author.clone())),
        );
        let valid_until = Timestamp::from_micros(100_000);
        let issuance_action = Action {
            header: ActionHeader {
                author: author.clone(),
                timestamp: Timestamp::from_micros(2),
                action_seq: 29_502,
                prev_action: Some(root.clone()),
            },
            data: ActionData::CreateLink(CreateLinkData {
                base_address: root.clone().into(),
                target_address: AnyLinkableHash::from(delegate.clone()),
                zome_index: 0.into(),
                link_type: 0.into(),
                tag: LinkTag::new(
                    [
                        b"head-delegation-issued:v1:".as_slice(),
                        &valid_until.as_micros().to_be_bytes(),
                    ]
                    .concat(),
                ),
            }),
        };
        let issuance_hash = ActionHash::with_data_sync(&issuance_action);
        let issuance = ChainOp::CreateLink(SignedAction::new(issuance_action, Signature([6; 64])));
        let prior_action = Action {
            header: ActionHeader {
                author: author.clone(),
                timestamp: Timestamp::from_micros(3),
                action_seq: 29_503,
                prev_action: Some(issuance_hash.clone()),
            },
            data: ActionData::Create(CreateData {
                entry_type: EntryType::App(AppEntryDef::new(
                    0.into(),
                    1.into(),
                    EntryVisibility::Public,
                )),
                entry_hash: EntryHash::from_raw_32(vec![7; 32]),
            }),
        };
        let prior_hash = ActionHash::with_data_sync(&prior_action);
        let prior = ChainOp::CreateRecord(
            SignedAction::new(prior_action, Signature([0; 64])),
            OpEntry::ActionOnly,
        );
        let head = root.clone();
        let signature = Signature([8; 64]);
        let witness_action = Action {
            header: ActionHeader {
                author: author.clone(),
                timestamp: Timestamp::from_micros(4),
                action_seq: 29_504,
                prev_action: Some(prior_hash.clone()),
            },
            data: ActionData::CreateLink(CreateLinkData {
                base_address: root.clone().into(),
                target_address: head.clone().into(),
                zome_index: 0.into(),
                link_type: 0.into(),
                tag: LinkTag::new(
                    [
                        b"head-acceptance:v2:".as_slice(),
                        signature.as_ref(),
                        head.get_raw_39(),
                    ]
                    .concat(),
                ),
            }),
        };
        let witness_hash = ActionHash::with_data_sync(&witness_action);
        let grant = DelegationEvidence {
            payload: DelegationPayloadEvidence {
                grantor: author.clone(),
                delegate: delegate.clone(),
                scope: "lesson-1".to_string(),
                valid_until,
                root_action_hash: root.clone(),
                dna_hash: dna.clone(),
                issuance_action_hash: Some(issuance_hash.clone()),
            },
            signature: signature.clone(),
            acceptance: AcceptanceEvidence {
                head_action_hash: head.clone(),
                witness_action_hash: witness_hash.clone(),
                accepted_at: Timestamp::from_micros(4),
            },
        };
        let mut election_tag = b"canonical-head:earned|delegation:".to_vec();
        election_tag.extend(holochain_serialized_bytes::encode(&grant).unwrap());
        let election_action = Action {
            header: ActionHeader {
                author: delegate,
                timestamp: Timestamp::from_micros(5),
                action_seq: 0,
                prev_action: None,
            },
            data: ActionData::CreateLink(CreateLinkData {
                base_address: root.clone().into(),
                target_address: head.clone().into(),
                zome_index: 0.into(),
                link_type: 0.into(),
                tag: LinkTag::new(election_tag),
            }),
        };
        let election_hash = ActionHash::with_data_sync(&election_action);
        let witness = ChainOp::CreateLink(SignedAction::new(witness_action, Signature([0; 64])));
        let election = ChainOp::CreateLink(SignedAction::new(election_action, Signature([0; 64])));
        assert!(verify_issuance_context(
            &issuance,
            &grant,
            &root_op,
            &root,
            &author,
            (0.into(), 0.into()),
        )
        .is_ok());
        let mut wrong_grant = DelegationEvidence {
            payload: DelegationPayloadEvidence {
                grantor: author.clone(),
                delegate: AgentPubKey::from_raw_32(vec![9; 32]),
                scope: "lesson-1".to_string(),
                valid_until,
                root_action_hash: root.clone(),
                dna_hash: dna.clone(),
                issuance_action_hash: Some(issuance_hash.clone()),
            },
            signature,
            acceptance: AcceptanceEvidence {
                head_action_hash: head.clone(),
                witness_action_hash: witness_hash.clone(),
                accepted_at: Timestamp::from_micros(4),
            },
        };
        assert!(verify_issuance_context(
            &issuance,
            &wrong_grant,
            &root_op,
            &root,
            &author,
            (0.into(), 0.into()),
        )
        .is_err());
        wrong_grant.payload.delegate = grant.payload.delegate.clone();
        wrong_grant.payload.valid_until = Timestamp::from_micros(2);
        assert!(verify_issuance_context(
            &issuance,
            &wrong_grant,
            &root_op,
            &root,
            &author,
            (0.into(), 0.into()),
        )
        .is_err());

        let ops: Vec<ChainOp> = vec![root_op, issuance, prior, witness, election];
        let timings: HashMap<_, _> = ops
            .iter()
            .map(|op| {
                let hash = op.to_hash();
                (
                    hash.clone(),
                    Timing {
                        op_hash: hash,
                        when_received: Timestamp::from_micros(10),
                        when_integrated: Some(Timestamp::from_micros(20)),
                        abandoned_at: None,
                        validation_status: Some(OpValidity::Accepted),
                        locally_validated: Some(true),
                    },
                )
            })
            .collect();
        let request = || Request {
            content_id: "lesson-1".to_string(),
            dna: dna.clone(),
            head: head.clone(),
            root: root.clone(),
            election_link: election_hash.clone(),
            issuance_action_hash: Some(issuance_hash.clone()),
            acceptance_witness_hash: Some(witness_hash.clone()),
            deadline_micros: 20,
            state_pages: vec![],
            timing_pages: vec![],
        };
        let full: Vec<_> = ops.iter().cloned().map(DhtOp::from).collect();
        let evidence = correlate(request(), full.clone(), timings.clone()).unwrap();
        assert_eq!(evidence.issuance.unwrap().action, issuance_hash);
        assert_eq!(
            evidence.acceptance_author_history.last().unwrap().action,
            issuance_hash
        );
        assert_eq!(evidence.acceptance_author_history.len(), 2);
        let without_issuer = vec![
            full[0].clone(),
            full[2].clone(),
            full[3].clone(),
            full[4].clone(),
        ];
        assert!(correlate(request(), without_issuer, timings.clone())
            .err()
            .unwrap()
            .to_string()
            .contains("issuance CreateLink not integrated"));
        let without_prior = vec![
            full[0].clone(),
            full[1].clone(),
            full[3].clone(),
            full[4].clone(),
        ];
        assert!(correlate(request(), without_prior, timings.clone())
            .err()
            .unwrap()
            .to_string()
            .contains("author history action not integrated"));
        let mut late = timings.clone();
        late.get_mut(&ops[3].to_hash()).unwrap().when_integrated = Some(Timestamp::from_micros(21));
        assert!(correlate(request(), full, late).is_err());
        assert!(required_acceptance(b"canonical-head:earned|delegation:invalid").is_err());
    }
}
