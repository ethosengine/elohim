//! Finding 6: a crossing custody Produce is bounded by the fold's per-event cap at put time.
use super::crossing_fits;
use elohim_epr_rea::{atom_cid, AgentRef, FlowEvent, FlowRecord, Magnitude, ReaVerb};

fn produce(filler: usize) -> FlowRecord {
    let output = eprfs_core::BlobCid::compute_bytes(b"artifact");
    FlowRecord::Event(FlowEvent {
        action: ReaVerb::Produce,
        provider: AgentRef("builder".into()),
        receiver: AgentRef("device".into()),
        resource: *output.as_cid(),
        quantity: Magnitude::Count {
            value: 8.0,
            unit: "bytes".into(),
        },
        process: None,
        in_scope_of: atom_cid(&"scope").expect("scope"),
        fulfills: vec![],
        satisfies: vec![],
        occurred_at: "2026-11-18T00:00:00Z".into(),
        classified_as: vec![
            "custody-blob".into(),
            output.to_string(),
            format!("env:requires-doc={}", "A".repeat(filler)),
        ],
    })
}

fn encoded(record: &FlowRecord) -> usize {
    let FlowRecord::Event(event) = record else {
        unreachable!("an event")
    };
    elohim_epr_rea::model::canonical_bytes(event).unwrap().len()
}

#[test]
fn a_crossing_produce_over_the_fold_cap_is_refused_and_one_at_it_is_recorded() {
    let cap = eprfs_peer::MAX_EVENT_BYTES;
    // An inline document at its own cap leaves the Produce well under the fold's.
    let usual = produce(super::requires::INLINE_MAX_ENCODED);
    assert!(encoded(&usual) < cap, "{}", encoded(&usual));
    assert!(crossing_fits(&usual).is_ok());
    // Grow the slot until the whole encoded event is exactly at, then one byte past, the cap.
    let overhead = encoded(&produce(0));
    let mut filler = cap - overhead;
    while encoded(&produce(filler)) > cap {
        filler -= 1;
    }
    while encoded(&produce(filler + 1)) <= cap {
        filler += 1;
    }
    assert_eq!(encoded(&produce(filler)), cap);
    assert!(crossing_fits(&produce(filler)).is_ok());
    let over = produce(filler + 1);
    assert!(encoded(&over) > cap);
    let why = crossing_fits(&over).unwrap_err().to_string();
    assert!(why.contains("per-event cap"), "{why}");
    assert!(why.contains(&format!("{cap}-byte")), "{why}");
}
