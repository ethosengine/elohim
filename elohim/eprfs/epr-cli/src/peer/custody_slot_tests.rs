//! Findings 1 and 2: a custody row carries its gaps, and a duplicated slot is no row.
use super::{artifact_from_event, Admission, HeadTable};
use elohim_epr_rea::{atom_cid, AgentRef, FlowEvent, Magnitude, ReaVerb};

/// A local custody Produce (no `via:`, received by `device`) carrying `extra` slots.
fn produce(extra: &[&str]) -> FlowEvent {
    let output = eprfs_core::BlobCid::compute_bytes(b"artifact");
    let mut classified_as = vec![
        "custody-blob".to_string(),
        output.to_string(),
        format!("env:key={}", eprfs_core::BlobCid::compute(b"key")),
        format!("env:output={output}"),
        "manifest:m".to_string(),
        "env:bytes=8".to_string(),
    ];
    classified_as.extend(extra.iter().map(|slot| (*slot).to_string()));
    FlowEvent {
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
        classified_as,
    }
}

fn admission() -> Admission {
    Admission::from_roster(&[], "root", &|_: &str, _: &[u8], _: &[u8]| false)
}

#[test]
fn a_custody_row_carries_the_recorders_gaps() {
    let (admission, heads) = (admission(), HeadTable::default());
    let event = produce(&[
        "env:commit=c1",
        "env:gaps=build-inputs.build-scripts,isa.x86.partial-decode",
    ]);
    let row = artifact_from_event(&"r", &event, "device", &admission, &heads).expect("row");
    assert_eq!(
        row.gaps,
        ["build-inputs.build-scripts", "isa.x86.partial-decode"]
    );
    let plain = artifact_from_event(
        &"r",
        &produce(&["env:commit=c1"]),
        "device",
        &admission,
        &heads,
    )
    .expect("row");
    assert!(plain.gaps.is_empty());
    // Two gap statements are a record inconsistent with itself, not a union.
    let twice = produce(&["env:commit=c1", "env:gaps=a", "env:gaps=isa.x86"]);
    assert!(artifact_from_event(&"r", &twice, "device", &admission, &heads).is_none());
}

#[test]
// The verdict counterpart needs an admitted sibling: tests/peer.rs
// peer_verdicts_reads_only_matching_folded_admitted_siblings.
fn a_duplicated_custody_slot_makes_no_row_rather_than_the_last_value() {
    let (admission, heads) = (admission(), HeadTable::default());
    for extra in [
        &["env:commit=c1", "env:commit=c2"][..],
        &["env:commit=c1", "env:bytes=9"],
        &["env:commit=c1", "env:agentId=a", "env:agentId=b"],
    ] {
        let event = produce(extra);
        assert!(
            artifact_from_event(&"r", &event, "device", &admission, &heads).is_none(),
            "{extra:?}"
        );
    }
}

#[test]
fn a_custody_row_carries_its_single_c_baseline_marker() {
    let (admission, heads) = (admission(), HeadTable::default());
    let row = |extra: &[&str]| {
        artifact_from_event(&"r", &produce(extra), "device", &admission, &heads).expect("row")
    };
    assert_eq!(
        row(&["env:commit=c1", "env:isa-c=x86-64"]).isa_c.as_deref(),
        Some("x86-64")
    );
    assert_eq!(
        row(&["env:commit=c1"]).isa_c,
        None,
        "a release from before the marker"
    );
    // Two markers are the producer's word twice, not a choice: neither stands.
    assert_eq!(
        row(&["env:commit=c1", "env:isa-c=x86-64", "env:isa-c=x86-64-v3"]).isa_c,
        None
    );
}
