//! The §2.F contract (interfaces spec, "A task placed on a berth"), over ark-core's `task.rs` and
//! placement-core's floor: each test names the contract clause it pins.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use ark_core::manifest::{ArtifactRef, ResourceQuota, Restart, RuntimeEnvelope, StdinSource};
use ark_core::{
    eprfs_depth, placement_depth, Berth, BerthOffer, EffectiveTier, ExitClass, PassphraseSource,
    Passport, ProcessPassport, RuntimeManifest, TaskChild, TaskKind, TaskOutcome, TaskSpec,
    MAX_OFFER_HOLDS,
};
use cid::Cid;
use elohim_epr::Reach;
use elohim_epr_rea::AgentRef;
use eprfs_core::{
    BlobCid, DerivationBasis, DerivationRecord, MethodDescriptor, ProjectedRoot, TreePath, Witness,
};
use placement_core::kind::testing::assert_scope_is_derived;
use placement_core::{
    admit, AgentRefKey, Candidate, Capacity, Consent, EprScope, FailureDomain, GrantScope, InForce,
    IneligibleReason, Isolation, KindId, NearestBytes, Need, NeedClass, NeedKey, NeedTerms,
    PlacementKind, RankInput, Ranking, Scratch, Weights,
};
use seam_contracts::harness::arbitrated::check_arbitration;
use seam_contracts::Answer;

const SHA: &str = "ffa40a0c6fab5ce062c4af76328dfe2de143256ddf791a504d72bca698a9ba20";
const NOW: u64 = 1_000;

fn child() -> TaskChild {
    TaskChild {
        artifact: ArtifactRef::Pinned {
            cid: None,
            sha256: SHA.into(),
            bytes: None,
        },
        argv: vec![
            "{artifact}".into(),
            "--input".into(),
            "{input}".into(),
            "--out".into(),
            "{output}".into(),
        ],
        env: BTreeMap::new(),
        env_scrub: true,
        stdin: StdinSource::Null,
        readiness: Vec::new(),
    }
}

fn envelope() -> RuntimeEnvelope {
    RuntimeEnvelope {
        bound: ResourceQuota {
            memory_bytes: Some(1 << 30),
            cpu_millis: Some(2_000),
            pids: Some(64),
            disk_bytes: Some(1 << 20),
        },
        ..Default::default()
    }
}

fn path(p: &str) -> TreePath {
    TreePath::from_slash_bytes(p.as_bytes()).unwrap()
}

/// The output selection every fixture spec makes, bound into its method's `exec`.
fn outputs() -> Vec<String> {
    vec!["report".into()]
}

/// The view size every fixture need and registration measures.
const VIEW: u64 = 4096;

fn method(name: &str, child: &TaskChild, envelope: Option<&RuntimeEnvelope>) -> MethodDescriptor {
    method_with(name, child, envelope, &outputs())
}

fn method_with(
    name: &str,
    child: &TaskChild,
    envelope: Option<&RuntimeEnvelope>,
    outputs: &[String],
) -> MethodDescriptor {
    MethodDescriptor {
        name: name.into(),
        tools: BTreeMap::from([("node".into(), "22".into())]),
        flags: Vec::new(),
        params: None,
        exec: Some(child.exec_cid(envelope, outputs).unwrap()),
        step: None,
        embedder: None,
        view: vec![path("genesis/a2o"), path("elohim/elohim-storage")],
        non_path_inputs: BTreeMap::from([("dnaHash".into(), "uhC0kX".into())]),
        deterministic: false,
    }
}

fn root(tag: &str) -> BlobCid {
    BlobCid::compute(tag.as_bytes())
}

fn spec_with(envelope: Option<RuntimeEnvelope>) -> TaskSpec {
    let c = child();
    TaskSpec::new(
        method("a2o-stage:x", &c, envelope.as_ref()),
        root("input"),
        Some(root("base")),
        c,
        envelope,
        outputs(),
    )
    .unwrap()
}

fn spec() -> TaskSpec {
    spec_with(Some(envelope()))
}

fn agent(name: &str) -> AgentRef {
    AgentRef(format!("uhCAk-{name}"))
}

fn space() -> Cid {
    elohim_epr::cid::compute_cid(b"space")
}

fn need_of(spec: &TaskSpec, carriers: Option<BTreeSet<AgentRefKey>>) -> Need {
    spec.as_need(
        agent("requester"),
        space(),
        Reach::Public,
        NeedClass::Member,
        carriers,
        VIEW,
    )
    .unwrap()
}

fn grant(scope: GrantScope) -> Consent {
    Consent::Granted {
        grant: "grant-1".into(),
        scope,
        recipient: AgentRefKey::of(&agent("requester")),
        valid_from: 0,
        valid_until: 10_000,
        epr_scope: EprScope::All,
    }
}

fn offer(carrier: &AgentRef, holds: Vec<(Cid, Reach)>, scopes: &[GrantScope]) -> BerthOffer {
    // The berth's own envelope: the task's bound, with room on disk for the materialized view too.
    let mut berth_envelope = envelope();
    berth_envelope.bound.disk_bytes = Some(1 << 24);
    let manifest = RuntimeManifest {
        envelope: Some(berth_envelope),
        ..Default::default()
    };
    let berth = Berth {
        manifest: "bafy-manifest".into(),
        incarnation: 3,
        ..Default::default()
    };
    BerthOffer::from_berth(
        &berth,
        elohim_epr::cid::compute_cid(carrier.0.as_bytes()),
        carrier.clone(),
        &manifest,
        &passport(&[EffectiveTier::Bounded]),
        holds,
        scopes.iter().copied().collect(),
    )
}

fn passport(tiers: &[EffectiveTier]) -> Passport {
    Passport {
        schema: 1,
        kind: "runtime-passport".into(),
        manifest: "bafy-manifest".into(),
        node: None,
        incarnation: 3,
        ark_version: "0.1.0".into(),
        processes: tiers
            .iter()
            .enumerate()
            .map(|(i, tier)| ProcessPassport {
                name: format!("p{i}"),
                artifact_sha256: SHA.into(),
                artifact_path: "/srv/ark/bin/secret-artifact".into(),
                pid: Some(42),
                started_at_epoch_ms: Some(1),
                ready: true,
                effective_tier: tier.clone(),
                deaths_in_window: 0,
            })
            .collect(),
        last_verdict: None,
        updated_at_epoch_ms: 1,
    }
}

fn candidate(offer: &BerthOffer, consent: Option<Consent>) -> Candidate {
    let receiver = AgentRefKey::of(&agent("requester"));
    Candidate {
        posture: offer.posture(Reach::Public, NOW),
        domain: FailureDomain::of(Some("A"), &offer.blade),
        live: Answer::Present(true),
        admitted: BTreeMap::from([(receiver.clone(), Answer::Present(Reach::Public))]),
        consent: BTreeMap::from([(receiver, consent.map_or(Answer::Absent, Answer::Present))]),
    }
}

fn kind_for(spec: &TaskSpec) -> TaskKind {
    TaskKind::new().with_spec(spec, Some(VIEW)).unwrap()
}

fn refusal(kind: &TaskKind, need: &Need, candidate: &Candidate) -> IneligibleReason {
    let domain = FailureDomain::of(Some("A"), &agent("requester"));
    admit(
        kind,
        need,
        candidate,
        Some(&domain),
        &Capacity::of_offer(&candidate.posture.offers[&KindId::Task]),
        NOW,
    )
    .map(|_| ())
    .unwrap_err()
    .why
    .why
}

// ---- the spec -----------------------------------------------------------------------------------

/// The `TaskSpec` CID is unmoved by an absent envelope (the rule of
/// `live_manifest_cid_is_unmoved_by_absent_envelope`), and so is the exec binding.
#[test]
fn an_absent_envelope_moves_no_cid() {
    let bare = spec_with(None);
    let json = serde_json::to_value(&bare).unwrap();
    assert!(json.get("envelope").is_none());

    // The same spec encoded without the field at all: byte-identical, so the CID is unmoved.
    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Before<'a> {
        method: &'a MethodDescriptor,
        input: &'a eprfs_core::TreeView,
        base_root: Option<&'a BlobCid>,
        child: &'a TaskChild,
        outputs: &'a [String],
    }
    let before = serde_ipld_dagcbor::to_vec(&Before {
        method: bare.method(),
        input: bare.input(),
        base_root: bare.base_root(),
        child: bare.child(),
        outputs: bare.outputs(),
    })
    .unwrap();
    assert_eq!(bare.cid().unwrap(), elohim_epr::cid::compute_cid(&before));

    #[derive(serde::Serialize)]
    struct ChildOnly<'a> {
        child: &'a TaskChild,
    }
    let child_only = serde_ipld_dagcbor::to_vec(&ChildOnly {
        child: bare.child(),
    })
    .unwrap();
    assert_eq!(
        bare.child().exec_cid(None, &[]).unwrap(),
        BlobCid::compute(&child_only)
    );

    // An envelope does move both.
    assert_ne!(spec().cid().unwrap(), bare.cid().unwrap());
    assert_ne!(
        bare.child().exec_cid(Some(&envelope()), &[]).unwrap(),
        bare.child().exec_cid(None, &[]).unwrap()
    );
}

/// `TaskSpec::new` refuses a child or envelope whose CID is not the descriptor's `exec`, and so does
/// decoding.
#[test]
fn new_and_decode_refuse_a_child_or_envelope_that_is_not_the_methods_exec() {
    let c = child();
    let m = method("a2o-stage:x", &c, Some(&envelope()));
    let mut other = c.clone();
    other.argv.push("--evil".into());
    // The method admits exactly this (child, envelope, outputs).
    assert!(TaskSpec::new(
        m.clone(),
        root("i"),
        None,
        c.clone(),
        Some(envelope()),
        outputs()
    )
    .is_ok());
    assert!(TaskSpec::new(
        m.clone(),
        root("i"),
        None,
        other,
        Some(envelope()),
        outputs()
    )
    .is_err());
    assert!(TaskSpec::new(m.clone(), root("i"), None, c.clone(), None, outputs()).is_err());
    let mut wider = envelope();
    wider.bound.memory_bytes = Some(1 << 40);
    assert!(TaskSpec::new(
        m.clone(),
        root("i"),
        None,
        c.clone(),
        Some(wider),
        outputs()
    )
    .is_err());
    let mut unbound = m.clone();
    unbound.exec = None;
    assert!(TaskSpec::new(
        unbound,
        root("i"),
        None,
        c.clone(),
        Some(envelope()),
        outputs()
    )
    .is_err());

    // Decoding runs the same check: a spec whose child was swapped after encoding is refused.
    let good = spec();
    let mut json = serde_json::to_value(&good).unwrap();
    assert_eq!(
        serde_json::from_value::<TaskSpec>(json.clone()).unwrap(),
        good
    );
    json["child"]["argv"][0] = "/bin/sh".into();
    assert!(serde_json::from_value::<TaskSpec>(json).is_err());

    // Decoding refuses an input whose paths are not the method's view.
    let mut json = serde_json::to_value(&good).unwrap();
    json["input"]["paths"] = serde_json::json!(["genesis"]);
    assert!(serde_json::from_value::<TaskSpec>(json).is_err());

    // And an unknown field.
    let mut json = serde_json::to_value(&good).unwrap();
    json["scope"] = "measure-stage".into();
    assert!(serde_json::from_value::<TaskSpec>(json).is_err());

    // The DAG-CBOR form round-trips too.
    let bytes = serde_ipld_dagcbor::to_vec(&good).unwrap();
    assert_eq!(
        serde_ipld_dagcbor::from_slice::<TaskSpec>(&bytes).unwrap(),
        good
    );
}

/// Two specs with the same method CID run the same program: the method binds (child, envelope), so
/// a second spec under it can differ only in what it reads.
#[test]
fn two_specs_with_one_method_cid_run_one_program() {
    let a = spec();
    let b = TaskSpec::new(
        a.method().clone(),
        root("another-input"),
        None,
        a.child().clone(),
        a.envelope().cloned(),
        outputs(),
    )
    .unwrap();
    assert_eq!(a.method().cid().unwrap(), b.method().cid().unwrap());
    assert_eq!(a.child_spec(), b.child_spec());
    assert_eq!(a.envelope(), b.envelope());
    assert_ne!(a.cid().unwrap(), b.cid().unwrap());
    let mut swapped = a.child().clone();
    swapped.env.insert("MODE".into(), "other".into());
    assert!(TaskSpec::new(
        a.method().clone(),
        root("x"),
        None,
        swapped,
        a.envelope().cloned(),
        outputs()
    )
    .is_err());
}

/// Specs selecting different outputs never share a derivation key (Codex review 2): the method's
/// `exec` binds the normalized selection, so the method CID, and with it the key, differs.
#[test]
fn different_output_selections_never_share_a_derivation_key() {
    let c = child();
    let env = envelope();
    let select = |outputs: &[&str]| {
        let outputs: Vec<String> = outputs.iter().map(|o| o.to_string()).collect();
        TaskSpec::new(
            method_with("a2o-stage:x", &c, Some(&env), &outputs),
            root("input"),
            None,
            c.clone(),
            Some(env.clone()),
            outputs,
        )
        .unwrap()
    };
    let projected: ProjectedRoot =
        serde_json::from_value(serde_json::json!(root("projected").to_string())).unwrap();
    let report = select(&["report"]);
    let logs = select(&["logs"]);
    assert_ne!(report.method().cid().unwrap(), logs.method().cid().unwrap());
    assert_ne!(
        report.derivation_key(&projected).unwrap(),
        logs.derivation_key(&projected).unwrap()
    );

    // A method bound to one selection refuses a spec making another, even under one child.
    assert!(TaskSpec::new(
        report.method().clone(),
        root("input"),
        None,
        c.clone(),
        Some(env.clone()),
        vec!["logs".into()],
    )
    .is_err());

    // Order and repetition do not move the selection: one normalized form, one key.
    let both = select(&["report", "logs"]);
    let shuffled = select(&["logs", "report", "logs"]);
    assert_eq!(both, shuffled);
    assert_eq!(both.outputs(), ["logs".to_string(), "report".to_string()]);
    assert_eq!(
        both.derivation_key(&projected).unwrap(),
        shuffled.derivation_key(&projected).unwrap()
    );

    // Decoding requires the normalized form, so a decoded spec's CID is the CID of its bytes.
    let mut json = serde_json::to_value(&both).unwrap();
    json["outputs"] = serde_json::json!(["report", "logs"]);
    assert!(serde_json::from_value::<TaskSpec>(json).is_err());
}

/// The task's child is temporary only (F12), and a task never inherits the ark's environment or
/// its passphrase, never runs a channel artifact, and never writes outside `{output}`.
#[test]
fn the_child_is_temporary_scrubbed_pinned_and_writes_only_under_output() {
    assert_eq!(
        child().to_child_spec().unwrap().policy.restart,
        Restart::Temporary
    );
    assert_eq!(spec().child_spec().policy.restart, Restart::Temporary);
    // A child outside a checked spec converts only when it passes the same checks (Codex review 1).
    let mut unsafe_child = child();
    unsafe_child.stdin = StdinSource::Passphrase;
    assert!(unsafe_child.to_child_spec().is_err());
    let mut unscrubbed = child();
    unscrubbed.env_scrub = false;
    assert!(unscrubbed.to_child_spec().is_err());
    let refuse = |c: TaskChild, outputs: Vec<String>| {
        let m = method_with("a2o-stage:x", &c, None, &outputs);
        TaskSpec::new(m, root("i"), None, c, None, outputs).is_err()
    };
    let mut c = child();
    c.stdin = StdinSource::Passphrase;
    assert!(refuse(c, vec![]));
    let mut c = child();
    c.env_scrub = false;
    assert!(refuse(c, vec![]));
    let mut c = child();
    c.artifact = ArtifactRef::Channel {
        channel_id: "stable".into(),
    };
    assert!(refuse(c, vec![]));
    let mut c = child();
    c.argv.clear();
    assert!(refuse(c, vec![]));
    let mut c = child();
    c.artifact = ArtifactRef::Pinned {
        cid: Some(elohim_epr::cid::compute_cid(b"not-the-digest").to_string()),
        sha256: SHA.into(),
        bytes: None,
    };
    assert!(refuse(c, vec![]));
    let mut c = child();
    c.artifact = ArtifactRef::Pinned {
        cid: Some(
            elohim_epr::cid::raw_cid_from_sha256_hex(SHA)
                .unwrap()
                .to_string(),
        ),
        sha256: SHA.into(),
        bytes: None,
    };
    assert!(!refuse(c, vec!["report/summary".into()]));
    for escape in ["", "/etc", "../up", "a/../../b", "./x", "a\\b"] {
        assert!(
            refuse(child(), vec![escape.into()]),
            "{escape:?} was admitted"
        );
    }
    // A raw (non-tree) input root is refused, and an empty view.
    let c = child();
    let raw = BlobCid::compute_raw(b"file");
    assert!(TaskSpec::new(
        method("m", &c, None),
        root("i"),
        None,
        c.clone(),
        None,
        outputs()
    )
    .is_ok());
    assert!(TaskSpec::new(method("m", &c, None), raw, None, c.clone(), None, outputs()).is_err());
    let mut viewless = method("m", &c, None);
    viewless.view.clear();
    assert!(TaskSpec::new(viewless, root("i"), None, c, None, outputs()).is_err());
}

/// The scope is derived one way from the method; the derivation key is (method CID, projected
/// input); the need is a {1,1} Task need whose requirements come from the envelope.
#[test]
fn the_need_scope_and_derivation_key_follow_from_the_spec() {
    let s = spec();
    assert_eq!(s.scope(), Some(GrantScope::MeasureStage));
    let c = child();
    let feedback = TaskSpec::new(
        method("brit-gate", &c, None),
        root("i"),
        None,
        c,
        None,
        outputs(),
    )
    .unwrap();
    assert_eq!(feedback.scope(), Some(GrantScope::SweettestFeedback));

    let projected: ProjectedRoot =
        serde_json::from_value(serde_json::json!(root("projected").to_string())).unwrap();
    let key = s.derivation_key(&projected).unwrap();
    assert_eq!(key.method, s.method().cid().unwrap());
    assert_eq!(key.input.cid(), projected.cid());

    let carriers = BTreeSet::from([AgentRefKey::of(&agent("jessica"))]);
    let need = need_of(&s, Some(carriers.clone()));
    assert_eq!(need.kind, KindId::Task);
    assert_eq!(need.key, NeedKey::Task(s.cid().unwrap()));
    assert_eq!((need.demand.holders, need.demand.domains), (1, 1));
    assert_eq!(need.requires.bytes, Some((1 << 20) + VIEW));
    assert_eq!(need.requires.memory_bytes, Some(1 << 30));
    assert_eq!(need.requires.cpu_millis, Some(2_000));
    assert_eq!(need.requires.pids, Some(64));
    assert_eq!(need.requires.carriers, Some(carriers));
    assert_eq!(need.receiver, agent("requester"));
    assert_eq!(need.in_scope_of, space());
    match &need.terms {
        NeedTerms::Task {
            input_root,
            scope,
            base_root,
        } => {
            assert_eq!(input_root.as_str(), root("input").to_string());
            assert_eq!(*scope, GrantScope::MeasureStage);
            assert_eq!(
                base_root.as_ref().unwrap().as_str(),
                root("base").to_string()
            );
        }
        NeedTerms::None => panic!("a task need carries task terms"),
    }
    // No envelope: the byte requirement is the view's size alone, never zero (Codex review 1).
    let bare = spec_with(None)
        .as_need(
            agent("r"),
            space(),
            Reach::Public,
            NeedClass::Member,
            None,
            VIEW,
        )
        .unwrap();
    assert_eq!(
        bare.requires,
        placement_core::Requirements {
            bytes: Some(VIEW),
            ..Default::default()
        }
    );
    // A disk bound plus a view size that overflows refuses instead of understating the total.
    let mut huge = envelope();
    huge.bound.disk_bytes = Some(u64::MAX);
    assert!(spec_with(Some(huge))
        .as_need(
            agent("r"),
            space(),
            Reach::Public,
            NeedClass::Member,
            None,
            1,
        )
        .is_err());
}

// ---- the berth offer ----------------------------------------------------------------------------

/// No `BerthOffer` built by `from_berth` contains a local path, port or passphrase, and no posture
/// lists a root below its own reach.
#[test]
fn a_berth_offer_carries_no_local_path_and_no_posture_names_a_root_below_its_reach() {
    let berth = Berth {
        manifest: "bafy-manifest".into(),
        node: Some("uhCAk-node".into()),
        data_root: PathBuf::from("/srv/elohim/secret-data-root"),
        passphrase: PassphraseSource::Literal("hunter2-passphrase".into()),
        ports: BTreeMap::from([("admin_ws".into(), 45_678)]),
        artifacts: BTreeMap::from([(
            "conductor".into(),
            PathBuf::from("/srv/elohim/artifact-path"),
        )]),
        incarnation: 7,
        ..Default::default()
    };
    let public = elohim_epr::cid::compute_cid(b"public-root");
    let intimate = elohim_epr::cid::compute_cid(b"intimate-root");
    let offer = BerthOffer::from_berth(
        &berth,
        elohim_epr::cid::compute_cid(b"berth"),
        agent("jessica"),
        &RuntimeManifest::default(),
        &passport(&[EffectiveTier::Enforced, EffectiveTier::Delegated]),
        vec![(public, Reach::Public), (intimate, Reach::Intimate)],
        BTreeSet::from([GrantScope::MeasureStage]),
    );
    let json = serde_json::to_string(&offer).unwrap();
    for secret in [
        "secret-data-root",
        "hunter2-passphrase",
        "45678",
        "artifact-path",
        "secret-artifact",
        "admin_ws",
    ] {
        assert!(!json.contains(secret), "the offer leaks {secret}: {json}");
    }
    assert_eq!(offer.incarnation, 7);
    assert_eq!(offer.manifest, "bafy-manifest");
    // The weakest tier across the passport, and no declared capacity without an envelope.
    assert_eq!(offer.isolation, Some(Isolation::Delegated));
    assert_eq!(offer.capacity, Capacity::default());
    assert_eq!(serde_json::from_str::<BerthOffer>(&json).unwrap(), offer);

    let public_posture = offer.posture(Reach::Public, NOW);
    let terms = &public_posture.offers[&KindId::Task];
    assert_eq!(terms.holds, vec![placement_core::CidString::from(&public)]);
    assert_eq!(terms.scopes, BTreeSet::from([GrantScope::MeasureStage]));
    assert_eq!(terms.isolation, Some(Isolation::Delegated));
    let intimate_posture = offer.posture(Reach::Intimate, NOW);
    assert_eq!(intimate_posture.offers[&KindId::Task].holds.len(), 2);
    assert_eq!(public_posture.carrier, agent("jessica"));
    assert_eq!(public_posture.reach, Reach::Public);

    // An empty passport is no isolation; a duplicate root keeps its narrower reach.
    let twice = BerthOffer::from_berth(
        &berth,
        elohim_epr::cid::compute_cid(b"berth"),
        agent("jessica"),
        &RuntimeManifest::default(),
        &passport(&[]),
        vec![(public, Reach::Public), (public, Reach::Intimate)],
        BTreeSet::new(),
    );
    assert_eq!(twice.isolation, None);
    assert_eq!(
        twice.holds(),
        [(placement_core::CidString::from(&public), Reach::Intimate)]
    );
    assert!(twice.posture(Reach::Public, NOW).offers[&KindId::Task]
        .holds
        .is_empty());
}

/// The holds are sorted and capped; capacity is the declared envelope bound, `disk_bytes` as bytes.
#[test]
fn holds_are_sorted_and_capped_and_capacity_is_the_declared_bound() {
    let holds: Vec<(Cid, Reach)> = (0..100u32)
        .map(|i| {
            (
                elohim_epr::cid::compute_cid(&i.to_be_bytes()),
                Reach::Public,
            )
        })
        .collect();
    let o = BerthOffer::from_berth(
        &Berth::default(),
        elohim_epr::cid::compute_cid(b"berth"),
        agent("b"),
        &RuntimeManifest {
            envelope: Some(envelope()),
            ..Default::default()
        },
        &passport(&[EffectiveTier::None]),
        holds,
        BTreeSet::new(),
    );
    assert_eq!(o.holds().len(), MAX_OFFER_HOLDS);
    assert!(o.holds().windows(2).all(|w| w[0].0 < w[1].0));
    assert_eq!(
        o.capacity,
        Capacity {
            bytes: Some(1 << 20),
            memory_bytes: Some(1 << 30),
            cpu_millis: Some(2_000),
            pids: Some(64),
        }
    );
    assert_eq!(o.isolation, Some(Isolation::None));
}

#[test]
fn the_isolation_and_depth_ladders_map_one_to_one() {
    for (tier, iso) in [
        (EffectiveTier::Enforced, Isolation::Enforced),
        (EffectiveTier::Bounded, Isolation::Bounded),
        (EffectiveTier::Delegated, Isolation::Delegated),
        (EffectiveTier::Intrinsic, Isolation::Intrinsic),
        (EffectiveTier::None, Isolation::None),
    ] {
        assert_eq!(Isolation::from(tier), iso);
    }
    for depth in [
        eprfs_core::Depth::Query,
        eprfs_core::Depth::Sparse,
        eprfs_core::Depth::OnRead,
        eprfs_core::Depth::Kept,
    ] {
        assert_eq!(eprfs_depth(placement_depth(depth)), depth);
        assert_eq!(
            serde_json::to_value(depth).unwrap(),
            serde_json::to_value(placement_depth(depth)).unwrap()
        );
    }
}

// ---- the kind -----------------------------------------------------------------------------------

/// The kind's scope and action follow the spec, never the need's declaration (F1), and its default
/// method is the built-in Task set's.
#[test]
fn the_task_kind_derives_its_scope_and_ranks_by_nearest_bytes() {
    let s = spec();
    let kind = kind_for(&s);
    let need = need_of(&s, None);
    assert_scope_is_derived(&kind, &need);
    assert_eq!(kind.scope(&need), Some(GrantScope::MeasureStage));
    assert!(kind.receiver_may_carry());
    assert_eq!(kind.default_method(), NearestBytes.method());
    assert_eq!(
        kind.default_method(),
        InForce::builtin(KindId::Task).set().method
    );
    assert!(kind.knows(&s.cid().unwrap()));
    assert!(!TaskKind::new().knows(&s.cid().unwrap()));
}

/// A requester declaring another scope than the method derives is refused for every carrier, and a
/// task whose spec this peer has not read is never carried.
#[test]
fn a_forged_scope_and_an_unread_spec_are_refused() {
    let s = spec();
    let mut forged = need_of(&s, None);
    if let NeedTerms::Task { scope, .. } = &mut forged.terms {
        *scope = GrantScope::SweettestFeedback;
    }
    let berth = offer(&agent("b"), vec![], GrantScope::ALL);
    let c = candidate(&berth, Some(grant(GrantScope::SweettestFeedback)));
    assert_eq!(
        refusal(&kind_for(&s), &forged, &c),
        IneligibleReason::Kind { label: "scope" }
    );
    // Unread: the kind derives no scope, so the declared one is refused first.
    let need = need_of(&s, None);
    let c = candidate(&berth, Some(grant(GrantScope::MeasureStage)));
    assert_eq!(
        refusal(&TaskKind::new(), &need, &c),
        IneligibleReason::Kind { label: "scope" }
    );
}

/// A berth without the derived scope is `Kind { "scope" }`; one with weaker isolation than the need
/// asks is `Kind { "isolation" }`; over capacity (pids included) is `OverCapacity`. Core and the
/// kind agree on each.
#[test]
fn scope_isolation_and_capacity_refusals() {
    let s = spec();
    let kind = kind_for(&s);
    let need = need_of(&s, None);
    let granted = Some(grant(GrantScope::MeasureStage));

    let no_scope = candidate(
        &offer(&agent("b"), vec![], &[GrantScope::SweettestFeedback]),
        granted.clone(),
    );
    assert_eq!(
        refusal(&kind, &need, &no_scope),
        IneligibleReason::Kind { label: "scope" }
    );
    let terms = &no_scope.posture.offers[&KindId::Task];
    assert_eq!(
        kind.fits(&need, &no_scope, terms, &Capacity::of_offer(terms)),
        Err(IneligibleReason::Kind { label: "scope" })
    );

    let mut strict = need.clone();
    strict.requires.isolation = Some(Isolation::Enforced);
    let bounded = candidate(
        &offer(&agent("b"), vec![], &[GrantScope::MeasureStage]),
        granted.clone(),
    );
    assert_eq!(
        refusal(&kind, &strict, &bounded),
        IneligibleReason::Kind { label: "isolation" }
    );
    let terms = &bounded.posture.offers[&KindId::Task];
    assert_eq!(
        kind.fits(&strict, &bounded, terms, &Capacity::of_offer(terms)),
        Err(IneligibleReason::Kind { label: "isolation" })
    );

    let mut hungry = need.clone();
    hungry.requires.pids = Some(65);
    assert_eq!(
        refusal(&kind, &hungry, &bounded),
        IneligibleReason::OverCapacity
    );
    assert_eq!(
        kind.fits(&hungry, &bounded, terms, &Capacity::of_offer(terms)),
        Err(IneligibleReason::OverCapacity)
    );

    // The honest need fits, charged exactly its requirements.
    let domain = FailureDomain::of(Some("A"), &agent("requester"));
    let eligible = admit(
        &kind,
        &need,
        &bounded,
        Some(&domain),
        &Capacity::of_offer(terms),
        NOW,
    )
    .unwrap();
    assert_eq!(eligible.charge().pids, 64);
    assert_eq!(eligible.charge().bytes, (1 << 20) + VIEW);
}

/// With no matching grant every remote berth is `NoConsent`; the requester's own envelope is never
/// `IsReceiver`; `--on` narrows every peer's admit (`NotNamed`).
#[test]
fn consent_receiver_and_named_carriers() {
    let s = spec();
    let kind = kind_for(&s);
    let need = need_of(&s, None);
    let remote = offer(&agent("b"), vec![], &[GrantScope::MeasureStage]);
    for consent in [
        None,
        Some(Consent::NotNeeded),
        Some(grant(GrantScope::SweettestFeedback)),
    ] {
        assert_eq!(
            refusal(&kind, &need, &candidate(&remote, consent.clone())),
            IneligibleReason::NoConsent,
            "{consent:?}"
        );
    }

    let own = offer(&agent("requester"), vec![], &[GrantScope::MeasureStage]);
    let own_candidate = candidate(&own, Some(Consent::NotNeeded));
    let domain = FailureDomain::of(Some("A"), &agent("requester"));
    let terms = &own_candidate.posture.offers[&KindId::Task];
    assert!(admit(
        &kind,
        &need,
        &own_candidate,
        Some(&domain),
        &Capacity::of_offer(terms),
        NOW
    )
    .is_ok());

    let on_jessica = need_of(
        &s,
        Some(BTreeSet::from([AgentRefKey::of(&agent("jessica"))])),
    );
    let granted = Some(grant(GrantScope::MeasureStage));
    assert_eq!(
        refusal(&kind, &on_jessica, &candidate(&remote, granted.clone())),
        IneligibleReason::NotNamed
    );
    assert_eq!(
        refusal(&kind, &on_jessica, &own_candidate),
        IneligibleReason::NotNamed
    );
    let jessica = offer(&agent("jessica"), vec![], &[GrantScope::MeasureStage]);
    let jc = candidate(&jessica, granted);
    let terms = &jc.posture.offers[&KindId::Task];
    assert!(admit(
        &kind,
        &on_jessica,
        &jc,
        Some(&domain),
        &Capacity::of_offer(terms),
        NOW
    )
    .is_ok());
}

/// Admits every candidate it can and ranks them with `NearestBytes`, best first, as carriers.
fn ranked(kind: &TaskKind, need: &Need, candidates: &[Candidate]) -> Vec<AgentRef> {
    let domain = FailureDomain::of(Some("A"), &agent("requester"));
    let eligible: Vec<_> = candidates
        .iter()
        .filter_map(|c| {
            let terms = &c.posture.offers[&KindId::Task];
            admit(
                kind,
                need,
                c,
                Some(&domain),
                &Capacity::of_offer(terms),
                NOW,
            )
            .ok()
        })
        .collect();
    let mut scratch = Scratch::default();
    scratch.prepare(candidates.iter().map(|c| c.agent().0.as_str()));
    let input_root = need.input_root();
    let mut out = Vec::new();
    NearestBytes.rank(
        &RankInput {
            need,
            receiver_domain: Some(&domain),
            input_root: input_root.as_ref(),
        },
        &eligible,
        &vec![false; eligible.len()],
        &Weights::new(),
        &scratch,
        &mut out,
    );
    out.iter().map(|&i| eligible[i].agent().clone()).collect()
}

/// `NearestBytes` puts a berth holding the input root above one holding only the base root, and that
/// above one holding neither, whatever the XOR order.
#[test]
fn nearest_bytes_prefers_the_input_holder_then_the_base_holder_whatever_the_xor_order() {
    let s = spec();
    let kind = kind_for(&s);
    let need = need_of(&s, None);
    let input = *root("input").as_cid();
    let base = *root("base").as_cid();
    let names = ["b1", "b2", "b3"];
    // Every assignment of the three roles to the three agents, so XOR never decides it.
    for perm in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let [holds_input, holds_base, holds_neither] = perm.map(|i| agent(names[i]));
        let candidates: Vec<Candidate> = [
            (&holds_neither, vec![]),
            (&holds_base, vec![(base, Reach::Public)]),
            (&holds_input, vec![(input, Reach::Public)]),
        ]
        .into_iter()
        .map(|(a, holds)| {
            candidate(
                &offer(a, holds, &[GrantScope::MeasureStage]),
                Some(grant(GrantScope::MeasureStage)),
            )
        })
        .collect();
        assert_eq!(
            ranked(&kind, &need, &candidates),
            vec![
                holds_input.clone(),
                holds_base.clone(),
                holds_neither.clone()
            ],
            "{perm:?}"
        );
    }
}

/// A single-holder task is a top-1 choice, so `check_arbitration` applies: the same carrier under
/// every input order, and the same on repeat.
#[test]
fn the_single_holder_choice_passes_the_arbitration_harness() {
    let s = spec();
    let kind = kind_for(&s);
    let need = need_of(&s, None);
    let base = *root("base").as_cid();
    let candidates: Vec<Candidate> = ["a", "b", "c", "d"]
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let holds = if i == 2 {
                vec![(base, Reach::Public)]
            } else {
                vec![]
            };
            candidate(
                &offer(&agent(n), holds, &[GrantScope::MeasureStage]),
                Some(grant(GrantScope::MeasureStage)),
            )
        })
        .collect();
    let report = check_arbitration(&candidates, |cs| {
        let top = ranked(&kind, &need, cs).into_iter().next()?;
        cs.iter().find(|c| c.agent() == &top).cloned()
    });
    assert!(report.is_ok(), "{report:?}");
    assert_eq!(ranked(&kind, &need, &candidates)[0], agent("c"));
}

// ---- the outcome --------------------------------------------------------------------------------

/// A failed run records no derivation; a clean one records its own output only.
#[test]
fn a_failed_run_records_no_derivation() {
    let s = spec();
    let task = s.cid().unwrap();
    let output = root("output");
    let projected: ProjectedRoot =
        serde_json::from_value(serde_json::json!(root("projected").to_string())).unwrap();
    let record = DerivationRecord {
        key: s.derivation_key(&projected).unwrap(),
        output: output.clone(),
        computed_by: "uhCAk-b".into(),
        basis: DerivationBasis::UnderGrant {
            grant: "grant-1".into(),
        },
        deterministic: false,
        witness: Witness::Signature { sig: "sig".into() },
    };
    let ok = TaskOutcome::new(
        task,
        ExitClass::Exited { code: 0 },
        Some(output.clone()),
        Some(record.clone()),
        "event".into(),
    )
    .unwrap();
    assert!(ok.succeeded());
    assert_eq!(ok.record(), Some(&record));
    assert_eq!(ok.output(), Some(&output));
    assert_eq!((ok.task(), ok.completion_event()), (&task, "event"));
    for exit in [
        ExitClass::Exited { code: 1 },
        ExitClass::Signaled {
            signal: 9,
            core_dumped: false,
        },
        ExitClass::OomKilled,
        ExitClass::Unknown,
    ] {
        assert!(TaskOutcome::new(
            task,
            exit,
            Some(output.clone()),
            Some(record.clone()),
            "event".into()
        )
        .is_err());
        let failed = TaskOutcome::new(task, exit, None, None, "event".into()).unwrap();
        assert!(!failed.succeeded() && failed.record().is_none());
    }
    assert!(TaskOutcome::new(
        task,
        ExitClass::Exited { code: 0 },
        Some(root("other")),
        Some(record.clone()),
        "event".into()
    )
    .is_err());
    assert!(TaskOutcome::new(
        task,
        ExitClass::Exited { code: 0 },
        None,
        Some(record),
        "event".into()
    )
    .is_err());
}

// ---- what the need declares is checked against the registered spec (Codex review 1) -------------

/// Lowering a need's requirements under an unchanged task key never admits it for less than the
/// spec's envelope: the kind charges at least the bound, and refuses where the bound does not fit.
#[test]
fn a_need_cannot_lower_its_requirements_below_the_specs_envelope() {
    let s = spec();
    let kind = kind_for(&s);
    let granted = Some(grant(GrantScope::MeasureStage));
    let domain = FailureDomain::of(Some("A"), &agent("requester"));

    let mut lowered = need_of(&s, None);
    lowered.requires.memory_bytes = None;
    lowered.requires.cpu_millis = Some(1);
    lowered.requires.pids = Some(1);
    lowered.requires.bytes = Some(1);
    assert_eq!(lowered.key, NeedKey::Task(s.cid().unwrap()));

    // Room for the envelope: admitted, charged the envelope's bound, not what the need claimed.
    let roomy = candidate(
        &offer(&agent("b"), vec![], &[GrantScope::MeasureStage]),
        granted.clone(),
    );
    let terms = &roomy.posture.offers[&KindId::Task];
    let charge = admit(
        &kind,
        &lowered,
        &roomy,
        Some(&domain),
        &Capacity::of_offer(terms),
        NOW,
    )
    .unwrap()
    .charge();
    assert_eq!(charge.memory_bytes, 1 << 30);
    assert_eq!(charge.cpu_millis, 2_000);
    assert_eq!(charge.pids, 64);
    // Bytes: the disk bound plus the registered view size, not the need's lowered claim.
    assert_eq!(charge.bytes, (1 << 20) + VIEW);

    // Room for the lowered need but not the envelope: refused, by core and by the kind alike.
    let mut tight = offer(&agent("b"), vec![], &[GrantScope::MeasureStage]);
    tight.capacity.pids = Some(32);
    let tight = candidate(&tight, granted.clone());
    assert_eq!(
        refusal(&kind, &lowered, &tight),
        IneligibleReason::OverCapacity
    );
    let terms = &tight.posture.offers[&KindId::Task];
    assert_eq!(
        kind.fits(&lowered, &tight, terms, &Capacity::of_offer(terms)),
        Err(IneligibleReason::OverCapacity)
    );

    // A task need with no byte requirement reserves no room for its view: never fits.
    let mut sizeless = need_of(&s, None);
    sizeless.requires.bytes = None;
    let terms = &roomy.posture.offers[&KindId::Task];
    assert_eq!(
        kind.fits(&sizeless, &roomy, terms, &Capacity::of_offer(terms)),
        Err(IneligibleReason::Unknown {
            input: "view-bytes"
        })
    );
    assert_eq!(
        refusal(&kind, &sizeless, &roomy),
        IneligibleReason::Unknown {
            input: "view-bytes"
        }
    );
}

/// A need naming another input or base root than its task's spec is the kind's own refusal.
#[test]
fn a_need_whose_roots_are_not_its_specs_is_refused() {
    let s = spec();
    let kind = kind_for(&s);
    let c = candidate(
        &offer(&agent("b"), vec![], &[GrantScope::MeasureStage]),
        Some(grant(GrantScope::MeasureStage)),
    );
    let terms = &c.posture.offers[&KindId::Task];
    let other = placement_core::CidString::from(root("elsewhere").as_cid());
    for forge in [0, 1, 2] {
        let mut need = need_of(&s, None);
        if let NeedTerms::Task {
            input_root,
            base_root,
            ..
        } = &mut need.terms
        {
            match forge {
                0 => *input_root = other.clone(),
                1 => *base_root = Some(other.clone()),
                _ => *base_root = None,
            }
        }
        assert_eq!(
            kind.fits(&need, &c, terms, &Capacity::of_offer(terms)),
            Err(IneligibleReason::Kind {
                label: ark_core::TASK_TERMS
            }),
            "forge {forge}"
        );
        assert_eq!(
            refusal(&kind, &need, &c),
            IneligibleReason::Kind {
                label: "task-terms"
            },
            "forge {forge}"
        );
    }
    let mut none = need_of(&s, None);
    none.terms = NeedTerms::None;
    assert!(kind
        .fits(&none, &c, terms, &Capacity::of_offer(terms))
        .is_err());
}

/// A pending task reserves its registered view before a higher-priority need is considered.
/// Lowering the public need's byte requirement cannot sell the same berth capacity twice.
#[test]
fn a_pending_task_reserves_its_trusted_view_before_earlier_classes() {
    use placement_core::{
        decide, Carrying, Charter, CidString, Demand, KindRegistry, Lease, MethodRegistry,
        RatificationRule, Snapshot,
    };

    let pending_spec = spec_with(None);
    let earlier_spec = TaskSpec::new(
        pending_spec.method().clone(),
        root("earlier-input"),
        None,
        child(),
        None,
        outputs(),
    )
    .unwrap();
    let mut pending = need_of(&pending_spec, None);
    pending.requires.bytes = Some(1);
    let mut earlier = need_of(&earlier_spec, None);
    earlier.class = NeedClass::Care;
    let mut berth = offer(&agent("b"), vec![], &[GrantScope::MeasureStage]);
    berth.capacity.bytes = Some(VIEW + 1);
    let carrier = candidate(&berth, Some(grant(GrantScope::MeasureStage)));
    let mut snapshot = Snapshot::empty(agent("b"));
    snapshot.now = NOW;
    snapshot.candidates = vec![carrier];
    snapshot.charters.insert(
        CidString::from(&space()),
        Charter {
            id: None,
            adopted_at: 0,
            demand: BTreeMap::new(),
            serve_order: vec![NeedClass::Care, NeedClass::Member],
            holding_floor: Demand {
                holders: 1,
                domains: 1,
            },
            forwarding: None,
            genesis_params: BTreeMap::new(),
            tuning_rule: RatificationRule::TouchedConsent,
            moves_per_event: (1, 4),
            freshness_horizon_secs: 600,
        },
    );
    snapshot.carrying = vec![Carrying {
        commitment: "pending-task".into(),
        need: pending.id().unwrap(),
        key: pending.key.clone(),
        kind: KindId::Task,
        carrier: agent("b"),
        receiver: pending.receiver.clone(),
        in_scope_of: space(),
        lease: Lease {
            from: 0,
            until: None,
        },
        under_params: *InForce::builtin(KindId::Task).ranking_cid(),
        honored: Answer::Present(false),
    }];

    // An unmeasured pending task still holds its room; unknown is never free capacity.
    for pending_view in [Some(VIEW), None] {
        let kinds = KindRegistry::new().with(Box::new(
            TaskKind::new()
                .with_spec(&pending_spec, pending_view)
                .unwrap()
                .with_spec(&earlier_spec, Some(VIEW))
                .unwrap(),
        ));
        for needs in [
            vec![pending.clone(), earlier.clone()],
            vec![earlier.clone(), pending.clone()],
        ] {
            snapshot.needs = needs;
            let plan = decide(
                &snapshot,
                &kinds,
                &MethodRegistry::builtin(),
                &mut Scratch::default(),
            );
            assert!(
                plan.adopt.is_empty(),
                "pending view {pending_view:?}: {plan:?}"
            );
            assert!(plan.unserved.iter().any(|u| u.key == earlier.key));
        }
    }
}

// ---- Codex review 2 -----------------------------------------------------------------------------

/// The byte floor is the disk bound plus the registered view size: a berth with room for the disk
/// bound alone is refused even for a need claiming less, a spec registered without a measured view
/// is never carried, and a sum that overflows refuses at registration.
#[test]
fn admission_charges_the_disk_bound_plus_the_registered_view() {
    let s = spec();
    let kind = kind_for(&s);
    let granted = Some(grant(GrantScope::MeasureStage));

    // Room for the disk bound, not for the disk bound plus the input.
    let mut disk_only = offer(&agent("b"), vec![], &[GrantScope::MeasureStage]);
    disk_only.capacity.bytes = Some(1 << 20);
    let disk_only = candidate(&disk_only, granted.clone());
    let terms = &disk_only.posture.offers[&KindId::Task];
    let mut lowered = need_of(&s, None);
    lowered.requires.bytes = Some(1);
    for need in [&lowered, &need_of(&s, None)] {
        assert_eq!(
            kind.fits(need, &disk_only, terms, &Capacity::of_offer(terms)),
            Err(IneligibleReason::OverCapacity)
        );
        assert_eq!(
            refusal(&kind, need, &disk_only),
            IneligibleReason::OverCapacity
        );
    }

    // Exactly the room the sum needs: admitted, charged the sum.
    let mut exact = offer(&agent("b"), vec![], &[GrantScope::MeasureStage]);
    exact.capacity.bytes = Some((1 << 20) + VIEW);
    let exact = candidate(&exact, granted.clone());
    let terms = &exact.posture.offers[&KindId::Task];
    let domain = FailureDomain::of(Some("A"), &agent("requester"));
    let charge = admit(
        &kind,
        &lowered,
        &exact,
        Some(&domain),
        &Capacity::of_offer(terms),
        NOW,
    )
    .unwrap()
    .charge();
    assert_eq!(charge.bytes, (1 << 20) + VIEW);

    // Registered without a measurement: refused for its view, whatever the need claims.
    let unmeasured = TaskKind::new().with_spec(&s, None).unwrap();
    assert!(unmeasured.knows(&s.cid().unwrap()));
    let roomy = candidate(
        &offer(&agent("b"), vec![], &[GrantScope::MeasureStage]),
        granted,
    );
    let terms = &roomy.posture.offers[&KindId::Task];
    assert_eq!(
        unmeasured.fits(
            &need_of(&s, None),
            &roomy,
            terms,
            &Capacity::of_offer(terms)
        ),
        Err(IneligibleReason::Unknown {
            input: "view-bytes"
        })
    );
    assert_eq!(
        refusal(&unmeasured, &need_of(&s, None), &roomy),
        IneligibleReason::Unknown {
            input: "view-bytes"
        }
    );

    // A disk bound plus a view size that overflows refuses at registration.
    let mut huge = envelope();
    huge.bound.disk_bytes = Some(u64::MAX);
    assert!(TaskKind::new()
        .with_spec(&spec_with(Some(huge)), Some(1))
        .is_err());
}

/// A berth offer read off the wire keeps the bounds `from_berth` gives it: more than
/// `MAX_OFFER_HOLDS` roots, a root at two reaches, and unsorted roots are all refused.
#[test]
fn a_decoded_berth_offer_is_bounded_and_lists_each_root_once() {
    let public = elohim_epr::cid::compute_cid(b"public-root");
    let good = offer(&agent("b"), vec![(public, Reach::Public)], &[]);
    let json = serde_json::to_value(&good).unwrap();
    assert_eq!(
        serde_json::from_value::<BerthOffer>(json.clone()).unwrap(),
        good
    );
    let entry = |cid: &Cid, reach: Reach| serde_json::json!([cid.to_string(), reach]);

    let mut many: Vec<Cid> = (0..=MAX_OFFER_HOLDS as u32)
        .map(|i| elohim_epr::cid::compute_cid(&i.to_be_bytes()))
        .collect();
    many.sort_by_key(|cid| cid.to_string());
    let mut oversized = json.clone();
    oversized["holds"] = many.iter().map(|c| entry(c, Reach::Public)).collect();
    assert!(serde_json::from_value::<BerthOffer>(oversized).is_err());
    let mut at_cap = json.clone();
    at_cap["holds"] = many[..MAX_OFFER_HOLDS]
        .iter()
        .map(|c| entry(c, Reach::Public))
        .collect();
    assert_eq!(
        serde_json::from_value::<BerthOffer>(at_cap)
            .unwrap()
            .holds()
            .len(),
        MAX_OFFER_HOLDS
    );

    let mut twice = json.clone();
    twice["holds"] = serde_json::json!([
        entry(&public, Reach::Intimate),
        entry(&public, Reach::Public)
    ]);
    assert!(serde_json::from_value::<BerthOffer>(twice).is_err());

    let mut unsorted = json.clone();
    unsorted["holds"] = serde_json::json!([
        entry(&many[1], Reach::Public),
        entry(&many[0], Reach::Public)
    ]);
    assert!(serde_json::from_value::<BerthOffer>(unsorted).is_err());

    let mut unknown = json;
    unknown["dataRoot"] = "/srv/elohim".into();
    assert!(serde_json::from_value::<BerthOffer>(unknown).is_err());
}

/// The berth offer's REA intents (interfaces spec §2.F, first instances): raised by the blade, in
/// scope of the berth's manifest, one `use` intent per declared resource with its own unit, and one
/// unquantified intent when nothing is declared.
#[test]
fn a_berth_offer_projects_one_use_intent_per_offered_resource() {
    use elohim_epr_rea::{Magnitude, ReaVerb};
    let manifest_cid = elohim_epr::cid::compute_cid(b"the-manifest");
    let berth = Berth {
        manifest: manifest_cid.to_string(),
        incarnation: 2,
        ..Default::default()
    };
    let berth_cid = elohim_epr::cid::compute_cid(b"berth");
    let project = |manifest: &RuntimeManifest| {
        BerthOffer::from_berth(
            &berth,
            berth_cid,
            agent("jessica"),
            manifest,
            &passport(&[EffectiveTier::Bounded]),
            vec![],
            BTreeSet::from([GrantScope::MeasureStage]),
        )
        .as_rea_intents()
        .unwrap()
    };
    let declared = project(&RuntimeManifest {
        envelope: Some(envelope()),
        ..Default::default()
    });
    let offered: Vec<(String, f64)> = declared
        .iter()
        .map(|intent| match &intent.resource_spec.quantity {
            Some(Magnitude::Count { value, unit }) => (unit.clone(), *value),
            other => panic!("an offered resource is a count, not {other:?}"),
        })
        .collect();
    assert_eq!(
        offered,
        vec![
            (ark_core::UNIT_DISK_BYTES.to_string(), (1u64 << 20) as f64),
            (ark_core::UNIT_MEMORY_BYTES.to_string(), (1u64 << 30) as f64),
            (ark_core::UNIT_CPU_MILLIS.to_string(), 2_000.0),
            (ark_core::UNIT_PIDS.to_string(), 64.0),
        ]
    );
    for intent in &declared {
        assert_eq!(intent.action, ReaVerb::Use);
        assert_eq!(intent.raised_by, agent("jessica"));
        assert_eq!(intent.in_scope_of, manifest_cid);
        assert_eq!(
            intent.resource_spec.classified_as,
            vec![
                ark_core::RUNTIME_TAG_BERTH_OFFER.to_string(),
                berth_cid.to_string()
            ]
        );
    }

    // Nothing declared: one offer to carry, its amount honestly absent.
    let bare = project(&RuntimeManifest::default());
    assert_eq!(bare.len(), 1);
    assert_eq!(bare[0].resource_spec.quantity, None);
    assert_eq!(bare[0].raised_by, agent("jessica"));

    // A manifest that is not a CID has no scope to project into.
    assert!(offer(&agent("b"), vec![], &[]).as_rea_intents().is_err());
}
