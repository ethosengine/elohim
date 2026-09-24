//! The fold attestation is appended to the log first, then renamed into the latest snapshot.
use elohim_epr::cid::compute_cid;
use elohim_epr_index::attest::{next_retry, record, LATEST_FILE, LOG_FILE};
use elohim_epr_rea::{AgentRef, FoldAttestation, FoldState, ShardManifest};

fn attestation(state: FoldState, at: i64) -> FoldAttestation {
    FoldAttestation {
        measure: compute_cid(b"measure"),
        shard: ShardManifest {
            arc: None,
            atoms: 0,
            bytes: 0,
            manifest: compute_cid(b"manifest"),
        },
        heads_at: Vec::new(),
        state,
        attested_by: AgentRef("(unclaimed)".into()),
        at,
    }
}

#[test]
fn record_appends_log_then_renames_latest() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("index/measure");
    let first = attestation(FoldState::Degraded { retried: 0 }, 1);
    let second = attestation(FoldState::Complete, 2);

    let first_cid = record(&store, &first).expect("recorded");
    assert_eq!(first_cid, first.cid().unwrap().to_string());
    assert_eq!(next_retry(&store), 1, "a degraded latest is retried next");
    let second_cid = record(&store, &second).expect("recorded");
    assert_eq!(second_cid, second.cid().unwrap().to_string());

    let log = std::fs::read_to_string(store.join(LOG_FILE)).unwrap();
    let lines: Vec<&str> = log.lines().collect();
    assert_eq!(lines.len(), 2, "the log keeps every act");
    assert_eq!(lines[0], serde_json::to_string(&first).unwrap());
    assert_eq!(lines[1], serde_json::to_string(&second).unwrap());

    let latest = std::fs::read_to_string(store.join(LATEST_FILE)).unwrap();
    assert_eq!(
        latest,
        serde_json::to_string_pretty(&second).unwrap() + "\n"
    );
    assert!(
        !store.join(format!("{LATEST_FILE}.tmp")).exists(),
        "the staged snapshot was renamed, not left beside it"
    );
    assert_eq!(
        next_retry(&store),
        0,
        "a complete latest resets the retry count"
    );
}
