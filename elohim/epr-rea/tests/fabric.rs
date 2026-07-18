//! Slice-1 fabric tests: DB-free folds over hand-built events (the rea-facing
//! test-first shape), the dev-pipeline mini-chain walk, and the sidecar floor.

use cid::Cid;
use elohim_epr_rea::{
    atom_cid, fulfillment, resource_state, AgentRef, Commitment, CommitmentState, FlowEvent,
    FlowRecord, FlowStore, FlowWalk, Intent, Magnitude, MemoryFlowStore, PinnedRef, Process,
    ReaVerb, ResourceSpec, SidecarFlowStore,
};

fn agent(name: &str) -> AgentRef {
    AgentRef(format!("uhCAk-test-{name}"))
}

/// Any content-addressed thing is a resource — here, a labeled test atom.
fn resource(label: &str) -> Cid {
    atom_cid(&label).expect("cid")
}

fn count(value: f64, unit: &str) -> Magnitude {
    Magnitude::Count {
        value,
        unit: unit.into(),
    }
}

fn event(
    action: ReaVerb,
    res: &Cid,
    quantity: Magnitude,
    scope: &Cid,
    process: Option<Cid>,
    fulfills: Vec<Cid>,
) -> FlowEvent {
    FlowEvent {
        action,
        provider: agent("claude"),
        receiver: agent("repo"),
        resource: *res,
        quantity,
        process,
        in_scope_of: *scope,
        fulfills,
        satisfies: vec![],
        occurred_at: "2026-07-18T12:00:00Z".into(),
    }
}

// ── CID identity ────────────────────────────────────────────────────────────────

#[test]
fn atom_cid_is_deterministic_and_content_sensitive() {
    let scope = resource("epic");
    let a = event(
        ReaVerb::Consume,
        &resource("spec"),
        count(1.0, "token"),
        &scope,
        None,
        vec![],
    );
    let b = a.clone();
    assert_eq!(atom_cid(&a).unwrap(), atom_cid(&b).unwrap());

    let mut c = a.clone();
    c.occurred_at = "2026-07-18T12:00:01Z".into();
    assert_ne!(atom_cid(&a).unwrap(), atom_cid(&c).unwrap());
}

// ── Folds ───────────────────────────────────────────────────────────────────────

#[test]
fn resource_state_folds_per_verb_and_unit_totals() {
    let scope = resource("epic");
    let spec_doc = resource("spec-doc");
    let other = resource("other-doc");
    let events = vec![
        event(
            ReaVerb::Consume,
            &spec_doc,
            count(1200.0, "token"),
            &scope,
            None,
            vec![],
        ),
        event(
            ReaVerb::Consume,
            &spec_doc,
            count(800.0, "token"),
            &scope,
            None,
            vec![],
        ),
        event(
            ReaVerb::Use,
            &spec_doc,
            count(2.0, "read"),
            &scope,
            None,
            vec![],
        ),
        // different resource — must not leak into the fold
        event(
            ReaVerb::Consume,
            &other,
            count(5000.0, "token"),
            &scope,
            None,
            vec![],
        ),
    ];

    let state = resource_state(&spec_doc, &events);
    assert_eq!(state.event_count, 3);
    assert_eq!(state.total(ReaVerb::Consume, "token"), 2000.0);
    assert_eq!(state.total(ReaVerb::Use, "read"), 2.0);
    assert_eq!(state.total(ReaVerb::Produce, "artifact"), 0.0);
}

#[test]
fn fulfillment_ratio_is_fulfilled_over_expected() {
    let scope = resource("epic");
    let joints = resource("dev-pipeline-joints");
    let commitment = Commitment {
        action: ReaVerb::Produce,
        provider: agent("claude"),
        receiver: agent("operator"),
        resource_spec: ResourceSpec {
            classified_as: vec!["dev:joint".into()],
            quantity: Some(count(6.0, "joint")),
        },
        in_scope_of: scope,
        valid_from: None,
        valid_until: None,
        state: CommitmentState::Active,
        satisfies: vec![],
    };
    let c_cid = atom_cid(&commitment).unwrap();

    let events = vec![
        event(
            ReaVerb::Produce,
            &joints,
            count(1.0, "joint"),
            &scope,
            None,
            vec![c_cid],
        ),
        event(
            ReaVerb::Produce,
            &joints,
            count(2.0, "joint"),
            &scope,
            None,
            vec![c_cid],
        ),
        // does not name the commitment — must not count
        event(
            ReaVerb::Produce,
            &joints,
            count(3.0, "joint"),
            &scope,
            None,
            vec![],
        ),
    ];

    let status = fulfillment(&c_cid, &commitment, &events);
    assert_eq!(status.event_count, 2);
    assert_eq!(status.fulfilled_quantity, 3.0);
    assert_eq!(status.expected_quantity, Some(6.0));
    assert_eq!(status.ratio(), Some(0.5));
}

// ── The dev-pipeline mini-chain walk ────────────────────────────────────────────

/// epic → (intent → commitment → produce spec-doc) → downstream scenario commitment.
struct MiniChain {
    store: MemoryFlowStore,
    epic: Cid,
    spec_doc: Cid,
    scenario: Cid,
    intent_cid: Cid,
    c1_cid: Cid,
    c2_cid: Cid,
    p2_cid: Cid,
}

fn build_mini_chain() -> MiniChain {
    let mut store = MemoryFlowStore::new();
    let epic = resource("epic");
    let spec_doc = resource("spec-doc");
    let scenario = resource("a2o-scenario");

    let intent = Intent {
        action: ReaVerb::Produce,
        resource_spec: ResourceSpec {
            classified_as: vec!["doc:spec".into()],
            quantity: None,
        },
        in_scope_of: epic,
        raised_by: agent("operator"),
    };
    let intent_cid = store.append(FlowRecord::Intent(intent)).unwrap();

    let c1 = Commitment {
        action: ReaVerb::Produce,
        provider: agent("claude"),
        receiver: agent("operator"),
        resource_spec: ResourceSpec {
            classified_as: vec!["doc:spec".into()],
            quantity: None,
        },
        in_scope_of: epic,
        valid_from: None,
        valid_until: None,
        state: CommitmentState::Active,
        satisfies: vec![intent_cid],
    };
    let c1_cid = store.append(FlowRecord::Commitment(c1)).unwrap();

    let p1 = Process {
        spec: PinnedRef {
            id: "elohim-dev-pipeline".into(),
            version: 1,
        },
        in_scope_of: epic,
        inputs: vec![epic],
        outputs: vec![spec_doc],
    };
    let _p1_cid = store.append(FlowRecord::Process(p1.clone())).unwrap();
    let p1_cid = atom_cid(&p1).unwrap();

    let e1 = event(
        ReaVerb::Produce,
        &spec_doc,
        count(1.0, "artifact"),
        &epic,
        Some(p1_cid),
        vec![c1_cid],
    );
    let mut e1 = e1;
    e1.satisfies = vec![intent_cid];
    store.append(FlowRecord::Event(e1)).unwrap();

    // Downstream: the scenario stage consumes the spec; its commitment is NOT yet fulfilled.
    let p2 = Process {
        spec: PinnedRef {
            id: "elohim-dev-pipeline".into(),
            version: 1,
        },
        in_scope_of: spec_doc,
        inputs: vec![spec_doc],
        outputs: vec![scenario],
    };
    let p2_cid = atom_cid(&p2).unwrap();
    store.append(FlowRecord::Process(p2)).unwrap();

    let c2 = Commitment {
        action: ReaVerb::Produce,
        provider: agent("claude"),
        receiver: agent("operator"),
        resource_spec: ResourceSpec {
            classified_as: vec!["a2o:scenario-green".into()],
            quantity: None,
        },
        in_scope_of: spec_doc,
        valid_from: None,
        valid_until: None,
        state: CommitmentState::Active,
        satisfies: vec![],
    };
    let c2_cid = store.append(FlowRecord::Commitment(c2)).unwrap();

    MiniChain {
        store,
        epic,
        spec_doc,
        scenario,
        intent_cid,
        c1_cid,
        c2_cid,
        p2_cid,
    }
}

#[test]
fn walk_back_finds_commitment_intent_and_inputs() {
    let chain = build_mini_chain();
    let lineage = chain.store.walk_back(&chain.spec_doc).unwrap();

    assert_eq!(lineage.producing_events.len(), 1);
    assert_eq!(lineage.processes.len(), 1);
    assert_eq!(lineage.inputs, vec![chain.epic]);
    assert_eq!(lineage.commitments, vec![chain.c1_cid]);
    assert_eq!(lineage.intents, vec![chain.intent_cid]);
}

#[test]
fn walk_forward_surfaces_dependents_and_unfulfilled_frontier() {
    let chain = build_mini_chain();
    let frontier = chain.store.walk_forward(&chain.spec_doc).unwrap();

    let dependent_cids: Vec<Cid> = frontier.dependents.iter().map(|(c, _)| *c).collect();
    assert_eq!(dependent_cids, vec![chain.p2_cid]);
    assert_eq!(frontier.outputs, vec![chain.scenario]);

    let unfulfilled_cids: Vec<Cid> = frontier.unfulfilled.iter().map(|(c, _)| *c).collect();
    assert_eq!(unfulfilled_cids, vec![chain.c2_cid]);
}

#[test]
fn fulfilled_commitment_leaves_the_frontier() {
    let mut chain = build_mini_chain();
    // The scenario goes green: an event fulfills c2.
    let e = event(
        ReaVerb::Produce,
        &chain.scenario,
        count(1.0, "verdict"),
        &chain.spec_doc,
        Some(chain.p2_cid),
        vec![chain.c2_cid],
    );
    chain.store.append(FlowRecord::Event(e)).unwrap();

    let frontier = chain.store.walk_forward(&chain.spec_doc).unwrap();
    assert!(frontier.unfulfilled.is_empty());
}

// ── Sidecar floor ───────────────────────────────────────────────────────────────

#[test]
fn sidecar_roundtrips_records_with_stable_cids() {
    let dir = tempfile::tempdir().unwrap();
    let scope = resource("epic");
    let e = event(
        ReaVerb::Consume,
        &resource("capability"),
        count(4242.0, "token"),
        &scope,
        None,
        vec![],
    );

    let (cid_a, cid_b) = {
        let mut store = SidecarFlowStore::open(dir.path()).unwrap();
        let a = store.append(FlowRecord::Event(e.clone())).unwrap();
        let b = store
            .append(FlowRecord::Commitment(Commitment {
                action: ReaVerb::Produce,
                provider: agent("claude"),
                receiver: agent("operator"),
                resource_spec: ResourceSpec {
                    classified_as: vec!["doc:spec".into()],
                    quantity: None,
                },
                in_scope_of: scope,
                valid_from: None,
                valid_until: None,
                state: CommitmentState::Proposed,
                satisfies: vec![],
            }))
            .unwrap();
        (a, b)
    };

    // Reopen fresh — records survive with identical CIDs, in append order.
    let store = SidecarFlowStore::open(dir.path()).unwrap();
    let records = store.records().unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].0, cid_a);
    assert_eq!(records[1].0, cid_b);
    assert!(matches!(records[0].1, FlowRecord::Event(_)));

    // The log lives at the named sidecar home.
    assert!(store.log_path().ends_with(".eprfs/status/flows.jsonl"));
    assert!(store.log_path().exists());
}

#[test]
fn sidecar_detects_tampered_lines() {
    let dir = tempfile::tempdir().unwrap();
    let scope = resource("epic");
    {
        let mut store = SidecarFlowStore::open(dir.path()).unwrap();
        store
            .append(FlowRecord::Event(event(
                ReaVerb::Consume,
                &resource("capability"),
                count(1.0, "token"),
                &scope,
                None,
                vec![],
            )))
            .unwrap();
    }

    let log = dir.path().join(".eprfs/status/flows.jsonl");
    let tampered = std::fs::read_to_string(&log)
        .unwrap()
        .replace("4242", "9999")
        .replace("\"value\":1.0", "\"value\":2.0");
    std::fs::write(&log, tampered).unwrap();

    let store = SidecarFlowStore::open(dir.path()).unwrap();
    assert!(
        store.records().is_err(),
        "tampered line must fail integrity"
    );
}
