//! Read-only correlation of fork dump-full-state and dump-op-timings pages.
//! Usage: correlate_head_ops request.json > receiver-integration.json
//! Capture bounded pages on EACH receiver's own admin port, with the pinned fork:
//! `hc-client call --port PORT dump-full-state DNA AGENT --limit 256 > state-0.json`
//! `hc-client call --port PORT dump-op-timings DNA --limit 256 > timings-0.json`
//! Continue with `--cursor WHEN_RECEIVED HASH` from, respectively,
//! `integration_dump.dht_ops_cursor` and `cursor` until all necessary ops are present.
//! Both cursors expose `when_received` and `hash`; retain every captured page.
//! Request JSON fields: head, root, election_link (raw 39-byte arrays), deadline_micros, state_pages,
//! timing_pages. Use the native observer's EXACT hashes and shared deadlineMs * 1000;
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
    head: ActionHash,
    root: ActionHash,
    election_link: ActionHash,
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
    head: ActionHash,
    root: ActionHash,
    election_link: ActionHash,
    deadline_micros: i64,
    history: Vec<ProvenOp>,
    election: ProvenOp,
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
    delegate: AgentPubKey,
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
    root: &ActionHash,
    head: &ActionHash,
    root_author: &AgentPubKey,
    election_type: (ZomeIndex, LinkType),
) -> Result<()> {
    let action = witness.signed_action();
    ensure!(
        action.data().author() == root_author,
        "acceptance witness has another author"
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
        link.base_address.clone().into_action_hash().as_ref() == Some(root)
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
            if matches!(op.as_ref(), ChainOp::CreateRecord(..)) {
                author_records.insert(action.clone(), op.clone());
            }
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
    let acceptance = if let Some(grant) = required_acceptance(&data.tag.0)? {
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
            (data.zome_index, data.link_type),
        )?;
        let mut current = witness.signed_action().data().clone();
        let revocation_tag = [
            b"head-delegation-revoked:v1:".as_slice(),
            grant.signature.as_ref(),
        ]
        .concat();
        // Match native 4096 total records, including the witness itself.
        for _ in 1..4096 {
            let previous = current
                .prev_action()
                .context("acceptance author history has no root")?;
            let prior = author_records
                .get(previous)
                .context("acceptance author history CreateRecord not integrated")?;
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
                kind: "CreateRecord:acceptance-author-history",
                timing: verified_timing(prior, &timings, request.deadline_micros)?,
            });
            if previous == &request.root {
                break;
            }
            current = action.data().clone();
        }
        ensure!(
            acceptance_author_history
                .last()
                .is_some_and(|op| op.action == request.root),
            "acceptance author history budget exceeded"
        );
        Some(ProvenOp {
            action: grant.acceptance.witness_action_hash,
            kind: "CreateLink:acceptance",
            timing: verified_timing(witness, &timings, request.deadline_micros)?,
        })
    } else {
        None
    };
    Ok(Evidence {
        head: request.head,
        root: request.root,
        election_link: request.election_link,
        deadline_micros: request.deadline_micros,
        history,
        election,
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
    fn delegated_capture_requires_native_witness_and_complete_author_history() {
        let (root_op, _) = fixture();
        let mut root_op = root_op;
        if let ChainOp::CreateRecord(_, entry) = &mut root_op {
            *entry = OpEntry::Present(Entry::Agent(AgentPubKey::from_raw_32(vec![1; 32])));
        }
        let root_action = root_op.signed_action().data().clone();
        let root = ActionHash::with_data_sync(&root_action);
        let head = root.clone();
        let signature = Signature([7; 64]);
        let mut prior_action = root_action.clone();
        prior_action.header.action_seq += 1;
        prior_action.header.prev_action = Some(root.clone());
        prior_action.header.timestamp = Timestamp::from_micros(2);
        let prior_hash = ActionHash::with_data_sync(&prior_action);
        let prior = ChainOp::CreateRecord(
            SignedAction::new(prior_action, Signature([0; 64])),
            OpEntry::ActionOnly,
        );
        let witness_action = Action {
            header: ActionHeader {
                author: root_action.author().clone(),
                timestamp: Timestamp::from_micros(3),
                action_seq: root_action.action_seq() + 2,
                prev_action: Some(prior_hash),
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
                delegate: AgentPubKey::from_raw_32(vec![2; 32]),
            },
            signature,
            acceptance: AcceptanceEvidence {
                head_action_hash: head.clone(),
                witness_action_hash: witness_hash.clone(),
                accepted_at: Timestamp::from_micros(3),
            },
        };
        let mut tag = b"canonical-head:earned|delegation:".to_vec();
        tag.extend(holochain_serialized_bytes::encode(&grant).unwrap());
        let mut election_action = witness_action.clone();
        election_action.header.author = AgentPubKey::from_raw_32(vec![2; 32]);
        election_action.header.timestamp = Timestamp::from_micros(4);
        if let ActionData::CreateLink(link) = &mut election_action.data {
            link.tag = LinkTag::new(tag);
        }
        let election_hash = ActionHash::with_data_sync(&election_action);
        let witness = ChainOp::CreateLink(SignedAction::new(witness_action, Signature([0; 64])));
        let election = ChainOp::CreateLink(SignedAction::new(election_action, Signature([0; 64])));
        assert!(verify_witness_context(
            &witness,
            &grant,
            &root,
            &head,
            root_action.author(),
            (0.into(), 0.into())
        )
        .is_ok());
        assert!(verify_witness_context(
            &witness,
            &grant,
            &root,
            &head,
            root_action.author(),
            (0.into(), 1.into())
        )
        .is_err());
        assert!(verify_witness_context(
            &witness,
            &grant,
            &root,
            &ActionHash::from_raw_32(vec![9; 32]),
            root_action.author(),
            (0.into(), 0.into())
        )
        .is_err());
        assert!(verify_witness_context(
            &witness,
            &grant,
            &root,
            &head,
            &grant.payload.delegate,
            (0.into(), 0.into())
        )
        .is_err());
        let ops = vec![root_op, witness, election, prior];
        let timings: HashMap<_, _> = ops
            .iter()
            .map(|op| {
                (
                    op.to_hash(),
                    Timing {
                        op_hash: op.to_hash(),
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
            head: head.clone(),
            root: root.clone(),
            election_link: election_hash.clone(),
            deadline_micros: 20,
            state_pages: vec![],
            timing_pages: vec![],
        };
        let full: Vec<_> = ops.iter().cloned().map(DhtOp::from).collect();
        assert!(correlate(request(), full.clone(), timings.clone()).is_ok());
        let without_witness = vec![full[0].clone(), full[2].clone()];
        assert!(correlate(request(), without_witness, timings.clone())
            .err()
            .unwrap()
            .to_string()
            .contains("acceptance CreateLink"));
        let without_author_history = full[..3].to_vec();
        assert!(
            correlate(request(), without_author_history, timings.clone())
                .err()
                .unwrap()
                .to_string()
                .contains("author history")
        );
        let mut late = timings.clone();
        late.get_mut(&ops[1].to_hash()).unwrap().when_integrated = Some(Timestamp::from_micros(21));
        assert!(correlate(request(), full, late).is_err());
        assert!(required_acceptance(b"canonical-head:earned|delegation:invalid").is_err());
    }
}
