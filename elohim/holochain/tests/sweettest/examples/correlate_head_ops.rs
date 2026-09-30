//! Read-only correlation of fork dump-full-state and dump-op-timings pages.
//! Usage: correlate_head_ops request.json > receiver-integration.json
//! Capture bounded pages on EACH receiver's own admin port, with the pinned fork:
//! `hc-client call --port PORT dump-full-state DNA AGENT --limit 256 > state-0.json`
//! `hc-client call --port PORT dump-op-timings DNA --limit 256 > timings-0.json`
//! Continue with `--cursor WHEN_RECEIVED HASH` from, respectively,
//! `integration_dump.dht_ops_cursor` and `cursor` until all necessary ops are present.
//! Both cursors expose `when_received` and `hash`; retain every captured page.
//! Request JSON fields: head, root, election_link, deadline_micros, state_pages,
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
    let mut links = HashMap::new();
    for op in ops {
        if let DhtOp::ChainOp(op) = op {
            let action = ActionHash::with_data_sync(op.signed_action().data());
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
    Ok(Evidence {
        head: request.head,
        root: request.root,
        election_link: request.election_link,
        deadline_micros: request.deadline_micros,
        history,
        election,
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
}
