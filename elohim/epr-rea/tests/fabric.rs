//! Slice-1 fabric tests: DB-free folds over hand-built events (the rea-facing
//! test-first shape), the dev-pipeline mini-chain walk, and the sidecar floor.

use cid::Cid;
use elohim_epr_rea::{
    atom_cid, fulfillment, resource_state, AgentRef, AlgedonicEvidence, Bound, Commitment,
    CommitmentState, FlowEvent, FlowRecord, FlowStore, FlowWalk, Intent, Magnitude,
    MemoryFlowStore, PinnedRef, Process, ReaVerb, ResourceSpec, Sense, SidecarFlowStore,
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
        bound: None,
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

// ── Limits live on commitments; pain flows against the promise ──────────────────

/// A promise to stay under a token ceiling. `bound: None` is the unbounded promise.
fn budget_commitment(scope: &Cid, bound: Option<Bound>) -> Commitment {
    Commitment {
        action: ReaVerb::Consume,
        provider: agent("claude"),
        receiver: agent("operator"),
        resource_spec: ResourceSpec {
            classified_as: vec!["dev:token-budget".into()],
            quantity: None,
        },
        in_scope_of: *scope,
        valid_from: None,
        valid_until: None,
        state: CommitmentState::Active,
        satisfies: vec![],
        bound,
    }
}

fn token_bound() -> Bound {
    Bound::new(1000.0, "token".into(), 85.0).expect("valid bound")
}

/// Spend `value` tokens against `commitment` — the DHT spells this edge `bounded_by`.
fn spend(scope: &Cid, commitment: Cid, value: f64) -> FlowEvent {
    event(
        ReaVerb::Consume,
        &resource("capability"),
        count(value, "token"),
        scope,
        None,
        vec![commitment],
    )
}

/// A floor bound is declarable and measurable today, but mints NO algedonic evidence — and
/// the silence is deliberate, not an oversight.
///
/// `AlgedonicEvidence` is ceiling-signed on the wire: its `crossed()` is `stock >= band_edge`,
/// and the shape is schema-pinned (`additionalProperties: false`) inside the DHT
/// FeedbackSignal kind whitelist. Pushing a floor breach through it would mint a signal whose
/// own `crossed()` reads FALSE — pain that denies itself, which is strictly worse than no
/// pain. Withholding is the honest-absence answer until the evidence becomes sense-aware,
/// which moves a wire shape and needs its own design gate.
#[test]
fn a_floor_bound_withholds_evidence_rather_than_inverting_it() {
    let scope = resource("epic");
    let floor = Bound::new(1000.0, "token".into(), 85.0)
        .expect("valid bound")
        .with_sense(Sense::Floor);
    let commitment = budget_commitment(&scope, Some(floor.clone()));
    let c_cid = atom_cid(&commitment).unwrap();

    // 40 of a 1000 floor is deep in breach territory for a floor…
    let events = vec![spend(&scope, c_cid, 40.0)];
    let status = fulfillment(&c_cid, &commitment, &events);
    assert!(
        status.pain.is_none(),
        "a floor must not mint ceiling-signed evidence"
    );

    // …and the bound itself DOES know it is breached. Only the signal is withheld, so this
    // is honest absence at the wire, never a claim that the floor is fine.
    assert!(floor.breached_by(40.0));
    assert!(floor.approached_by(1100.0));
    assert_eq!(floor.band_edge(), 1150.0);
}

#[test]
fn stock_crossing_the_band_edge_yields_approach_pain_against_the_commitment() {
    let scope = resource("epic");
    let commitment = budget_commitment(&scope, Some(token_bound()));
    let c_cid = atom_cid(&commitment).unwrap();
    let events = vec![spend(&scope, c_cid, 600.0), spend(&scope, c_cid, 280.0)];

    let status = fulfillment(&c_cid, &commitment, &events);
    match status.pain.expect("880 of 1000 is past the 85% band edge") {
        AlgedonicEvidence::Approach {
            stock,
            limit,
            bound_ref,
            threshold_pct,
        } => {
            assert_eq!(stock, 880.0);
            assert_eq!(limit, 1000.0);
            assert_eq!(threshold_pct, 85.0);
            // The bound IS the commitment: bound_ref is its CID, nothing else.
            assert_eq!(bound_ref, c_cid.to_string());
        }
        other => panic!("expected approach evidence, got {other:?}"),
    }
}

#[test]
fn stock_crossing_the_limit_yields_breach_pain() {
    let scope = resource("epic");
    let commitment = budget_commitment(&scope, Some(token_bound()));
    let c_cid = atom_cid(&commitment).unwrap();
    let events = vec![spend(&scope, c_cid, 900.0), spend(&scope, c_cid, 300.0)];

    let pain = fulfillment(&c_cid, &commitment, &events)
        .pain
        .expect("1200 of 1000 is a breach");
    assert_eq!(
        pain,
        AlgedonicEvidence::Breach {
            stock: 1200.0,
            limit: 1000.0,
            bound_ref: c_cid.to_string(),
        },
        "breach evidence carries no band edge — the line itself is the edge"
    );
}

#[test]
fn stock_inside_the_band_is_silence() {
    let scope = resource("epic");
    let commitment = budget_commitment(&scope, Some(token_bound()));
    let c_cid = atom_cid(&commitment).unwrap();
    let events = vec![spend(&scope, c_cid, 840.0)];

    assert!(
        fulfillment(&c_cid, &commitment, &events).pain.is_none(),
        "84% of the limit has not reached the 85% band edge"
    );
}

/// Honest absence: a promise that declared no ceiling can never be in pain, however much
/// flows against it. No bound, no limit, nothing to report.
#[test]
fn a_commitment_with_no_bound_never_yields_pain() {
    let scope = resource("epic");
    let commitment = budget_commitment(&scope, None);
    let c_cid = atom_cid(&commitment).unwrap();
    let events = vec![spend(&scope, c_cid, 9_000_000.0)];

    let status = fulfillment(&c_cid, &commitment, &events);
    assert_eq!(status.event_count, 1, "the flow is still observed…");
    assert!(
        status.pain.is_none(),
        "…but an unbounded promise has no line"
    );
}

/// Stock is what flowed against THIS promise, in the bound's own unit.
#[test]
fn stock_counts_only_events_bounded_by_this_commitment_in_the_bound_unit() {
    let scope = resource("epic");
    let commitment = budget_commitment(&scope, Some(token_bound()));
    let c_cid = atom_cid(&commitment).unwrap();
    let other = atom_cid(&"other-commitment").unwrap();

    let events = vec![
        spend(&scope, c_cid, 900.0),
        // Names a different commitment — not this promise's stock.
        spend(&scope, other, 900.0),
        // Names this commitment but in another unit — not this bound's stock.
        event(
            ReaVerb::Consume,
            &resource("capability"),
            count(900.0, "second"),
            &scope,
            None,
            vec![c_cid],
        ),
    ];

    let pain = fulfillment(&c_cid, &commitment, &events)
        .pain
        .expect("900 of 1000 is past the band edge");
    assert_eq!(pain.stock(), 900.0, "only the 900 tokens on this promise");
    assert!(
        matches!(pain, AlgedonicEvidence::Approach { .. }),
        "900 < 1000 — approaching, not breached"
    );
}

/// The projection: open pain across the store, keyed by the commitment CID.
#[test]
fn open_pain_projects_per_commitment_and_stays_silent_where_there_is_no_bound() {
    let mut store = MemoryFlowStore::new();
    let scope = resource("epic");

    let hurting = budget_commitment(&scope, Some(token_bound()));
    let hurting_cid = store.append(FlowRecord::Commitment(hurting)).unwrap();

    // Same bound, different scope ⇒ a distinct commitment — and it is nowhere near its line.
    let calm_scope = resource("other-epic");
    let calm = budget_commitment(&calm_scope, Some(token_bound()));
    let calm_cid = store.append(FlowRecord::Commitment(calm)).unwrap();

    let unbounded_scope = resource("third-epic");
    let unbounded = budget_commitment(&unbounded_scope, None);
    let unbounded_cid = store.append(FlowRecord::Commitment(unbounded)).unwrap();

    store
        .append(FlowRecord::Event(spend(&scope, hurting_cid, 1200.0)))
        .unwrap();
    store
        .append(FlowRecord::Event(spend(&calm_scope, calm_cid, 10.0)))
        .unwrap();
    store
        .append(FlowRecord::Event(spend(
            &unbounded_scope,
            unbounded_cid,
            50_000.0,
        )))
        .unwrap();

    let pain = store.open_pain().unwrap();
    assert_eq!(pain.len(), 1, "exactly one promise is in pain: {pain:?}");
    let (cid, evidence) = &pain[0];
    assert_eq!(*cid, hurting_cid, "pain is keyed by the commitment CID");
    assert_eq!(evidence.bound_ref(), hurting_cid.to_string());
    assert_eq!(evidence.stock(), 1200.0);
}

/// A promise that is no longer open cannot be in open pain — same rule the unfulfilled
/// frontier already applies.
#[test]
fn open_pain_ignores_commitments_that_are_no_longer_open() {
    let mut store = MemoryFlowStore::new();
    let scope = resource("epic");

    let mut revoked = budget_commitment(&scope, Some(token_bound()));
    revoked.state = CommitmentState::Revoked;
    let revoked_cid = store.append(FlowRecord::Commitment(revoked)).unwrap();
    store
        .append(FlowRecord::Event(spend(&scope, revoked_cid, 5000.0)))
        .unwrap();

    assert!(store.open_pain().unwrap().is_empty());
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
        bound: None,
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
        bound: None,
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
                bound: None,
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
