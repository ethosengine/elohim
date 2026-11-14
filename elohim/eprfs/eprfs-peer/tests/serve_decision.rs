//! The serve decision and the reach fold (host-compatibility plan item K12, packet 1): one
//! positive case is new, an admitted device of the person drawing a record whose own events
//! declare exactly `self` with no row for its method that leaves the steward out. Every other
//! combination is today's behaviour or a refusal.

use elohim_epr_rea::{AgentRef, FlowEvent, Magnitude, ReaVerb};
use eprfs_core::BlobCid;
use eprfs_peer::{
    event_reach, parse_reach, reach_claims, serve_decision, ArtifactServeList, Basis,
    DeclaredReach, Deny, ObjectKind, ReachClaim, Relationship, ServePolicy, REACH_LEVELS,
};
use serde_json::json;

const METHOD: &str = "mesh-build";
const ME: &str = "did:key:z6MkOwnDevice";
const SIBLING: &str = "did:key:z6MkSiblingDevice";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Row {
    Absent,
    Steward,
    /// A row for the method that leaves the steward out: an empty list.
    Excludes,
    /// A row for the method naming only a reserved carrier audience: it also leaves the steward out.
    CarrierOnly,
    /// The steward and a carrier: still names the steward.
    StewardAndCarrier,
    /// A steward row for a different method only: the method itself has no row.
    OtherMethod,
}

const CARRIER: &str = "carrier:did:key:z6MkCarrierDevice";

impl Row {
    /// The rows that name the steward audience for the method.
    fn grants(self) -> bool {
        matches!(self, Row::Steward | Row::StewardAndCarrier)
    }

    /// The rows that exist for the method and leave the steward out.
    fn narrows(self) -> bool {
        matches!(self, Row::Excludes | Row::CarrierOnly)
    }
}

fn serves(row: Row, disclose: bool) -> ArtifactServeList {
    let mut artifacts = serde_json::Map::new();
    match row {
        Row::Absent => {}
        Row::Steward => {
            artifacts.insert("serves".into(), json!({ METHOD: ["steward"] }));
        }
        Row::Excludes => {
            artifacts.insert("serves".into(), json!({ METHOD: [] }));
        }
        Row::CarrierOnly => {
            artifacts.insert("serves".into(), json!({ METHOD: [CARRIER] }));
        }
        Row::StewardAndCarrier => {
            artifacts.insert("serves".into(), json!({ METHOD: ["steward", CARRIER] }));
        }
        Row::OtherMethod => {
            artifacts.insert("serves".into(), json!({ "sysroot-pack": ["steward"] }));
        }
    }
    if disclose {
        artifacts.insert("discloseMethod".into(), json!([METHOD]));
    }
    ArtifactServeList::from_policy(&json!({ "berth": { "artifacts": artifacts } }))
        .expect("policy parses")
}

const OWN: ReachClaim = ReachClaim::Declared(DeclaredReach::OwnDevices);
const PRIVATE: ReachClaim = ReachClaim::Declared(DeclaredReach::Private);
const UNREADABLE: ReachClaim = ReachClaim::Declared(DeclaredReach::Unreadable);

/// One row of the design's truth table (k12 design §3.2): the policy (`None` = unavailable), the
/// requester, the claim, the object, and the outcome.
struct Case {
    row: u8,
    policy: Option<(Row, bool)>,
    who: Relationship,
    claim: ReachClaim,
    object: ObjectKind,
    outcome: Result<Basis, Deny>,
}

fn case(
    row: u8,
    policy: Option<(Row, bool)>,
    who: Relationship,
    claim: ReachClaim,
    object: ObjectKind,
    outcome: Result<Basis, Deny>,
) -> Case {
    Case {
        row,
        policy,
        who,
        claim,
        object,
        outcome,
    }
}

fn decide(
    policy: Option<(Row, bool)>,
    who: Relationship,
    claim: ReachClaim,
    object: ObjectKind,
) -> Result<Basis, Deny> {
    let list = policy.map(|(row, disclose)| serves(row, disclose));
    let policy = match &list {
        Some(list) => ServePolicy::Loaded(list),
        None => ServePolicy::Unavailable,
    };
    serve_decision(who, &claim, METHOD, object, &policy)
}

fn custody(output: &BlobCid, manifest: &BlobCid, receiver: &str, extra: &[&str]) -> FlowEvent {
    let mut classified_as = vec![
        "custody-blob".to_string(),
        output.to_string(),
        "copy:self".to_string(),
        format!("manifest:{manifest}"),
    ];
    classified_as.extend(extra.iter().map(ToString::to_string));
    FlowEvent {
        action: ReaVerb::Produce,
        provider: AgentRef(receiver.to_string()),
        receiver: AgentRef(receiver.to_string()),
        resource: *output.as_cid(),
        quantity: Magnitude::Count {
            value: 1.0,
            unit: "bytes".into(),
        },
        process: None,
        in_scope_of: *BlobCid::compute(b"scope").as_cid(),
        fulfills: Vec::new(),
        satisfies: Vec::new(),
        occurred_at: "2026-11-14T00:00:00Z".into(),
        classified_as,
    }
}

/// The claim the fold gives `manifest` over `events`, read as this device.
fn claim_of(events: &[FlowEvent], manifest: &BlobCid) -> Option<ReachClaim> {
    reach_claims(events, ME).get(manifest).copied()
}

fn blobs() -> (BlobCid, BlobCid) {
    (BlobCid::compute(b"output"), BlobCid::compute(b"manifest"))
}

#[test]
fn the_truth_table_holds_row_by_row() {
    use ObjectKind::{ManifestOrChunk as Bytes, MethodOrInput as Method};
    use Relationship::{None as Stranger, OwnDevice as Own};
    let slotless = ReachClaim::Slotless;
    let mut cases = vec![
        case(
            1,
            Some((Row::Steward, true)),
            Stranger,
            OWN,
            Bytes,
            Err(Deny::NotAdmitted),
        ),
        case(
            1,
            Some((Row::Steward, true)),
            Stranger,
            slotless,
            Method,
            Err(Deny::NotAdmitted),
        ),
        case(2, None, Own, OWN, Bytes, Err(Deny::PolicyUnavailable)),
        case(2, None, Own, slotless, Bytes, Err(Deny::PolicyUnavailable)),
        case(
            3,
            Some((Row::Steward, false)),
            Own,
            slotless,
            Bytes,
            Ok(Basis::MethodRow),
        ),
        case(
            4,
            Some((Row::Absent, true)),
            Own,
            slotless,
            Bytes,
            Err(Deny::NoMethodRow),
        ),
        case(
            4,
            Some((Row::Absent, true)),
            Own,
            slotless,
            Method,
            Err(Deny::NoMethodRow),
        ),
        case(
            5,
            Some((Row::Excludes, true)),
            Own,
            slotless,
            Bytes,
            Err(Deny::NoMethodRow),
        ),
        case(
            6,
            Some((Row::Steward, true)),
            Own,
            slotless,
            Method,
            Ok(Basis::MethodRow),
        ),
        case(
            7,
            Some((Row::Steward, false)),
            Own,
            slotless,
            Method,
            Err(Deny::MethodNotDisclosed),
        ),
        case(
            8,
            Some((Row::Absent, false)),
            Own,
            OWN,
            Bytes,
            Ok(Basis::Reach),
        ),
        case(
            9,
            Some((Row::Steward, false)),
            Own,
            OWN,
            Bytes,
            Ok(Basis::Reach),
        ),
        case(
            10,
            Some((Row::Excludes, true)),
            Own,
            OWN,
            Bytes,
            Err(Deny::Narrowed),
        ),
        case(
            10,
            Some((Row::Excludes, true)),
            Own,
            OWN,
            Method,
            Err(Deny::Narrowed),
        ),
        case(
            11,
            Some((Row::Absent, false)),
            Own,
            OWN,
            Method,
            Ok(Basis::Reach),
        ),
        case(
            11,
            Some((Row::Steward, false)),
            Own,
            OWN,
            Method,
            Ok(Basis::Reach),
        ),
        case(
            11,
            Some((Row::Steward, true)),
            Own,
            OWN,
            Method,
            Ok(Basis::Reach),
        ),
        case(
            12,
            Some((Row::Steward, true)),
            Own,
            PRIVATE,
            Bytes,
            Err(Deny::Private),
        ),
        case(
            12,
            Some((Row::Absent, false)),
            Own,
            PRIVATE,
            Method,
            Err(Deny::Private),
        ),
        case(
            13,
            Some((Row::Steward, true)),
            Own,
            UNREADABLE,
            Bytes,
            Err(Deny::Private),
        ),
    ];
    for openness in 3..=8 {
        let wider = ReachClaim::Declared(DeclaredReach::Wider(openness));
        cases.push(case(
            14,
            Some((Row::Steward, true)),
            Own,
            wider,
            Bytes,
            Err(Deny::LadderUnbuilt),
        ));
        cases.push(case(
            14,
            Some((Row::Absent, false)),
            Own,
            wider,
            Method,
            Err(Deny::LadderUnbuilt),
        ));
    }
    for c in &cases {
        assert_eq!(
            decide(c.policy, c.who, c.claim, c.object),
            c.outcome,
            "truth-table row {}: {:?} {:?} {:?} {:?}",
            c.row,
            c.policy,
            c.who,
            c.claim,
            c.object
        );
    }

    let (output, manifest) = blobs();
    let steward = Some((Row::Steward, false));
    // Row 15: slotless and slotted events of one manifest: the slotted events decide.
    let mixed = [
        custody(&output, &manifest, ME, &[]),
        custody(&output, &manifest, ME, &["env:reach=private"]),
        custody(&output, &manifest, ME, &[]),
    ];
    let claim = claim_of(&mixed, &manifest).expect("own custody");
    assert_eq!(
        decide(steward, Own, claim, Bytes),
        Err(Deny::Private),
        "row 15"
    );
    // Row 16: slotted events that disagree: the narrowest decides.
    let disagree = [
        custody(&output, &manifest, ME, &["env:reach=self"]),
        custody(&output, &manifest, ME, &["env:reach=private"]),
    ];
    let claim = claim_of(&disagree, &manifest).expect("own custody");
    assert_eq!(
        decide(steward, Own, claim, Bytes),
        Err(Deny::Private),
        "row 16"
    );
    // Row 17: no own custody event names the manifest: no claim, refused before any decision.
    let sibling = [custody(&output, &manifest, SIBLING, &["env:reach=self"])];
    assert_eq!(claim_of(&sibling, &manifest), None, "row 17");
    // Row 18: a device the roster dropped is no longer an own device at its next request.
    assert_eq!(
        decide(steward, Stranger, OWN, Bytes),
        Err(Deny::NotAdmitted),
        "row 18"
    );
}

#[test]
fn every_unlisted_combination_denies() {
    let rows = [
        Row::Absent,
        Row::Steward,
        Row::Excludes,
        Row::CarrierOnly,
        Row::StewardAndCarrier,
        Row::OtherMethod,
    ];
    let mut policies: Vec<Option<(Row, bool)>> = vec![None];
    for row in rows {
        policies.push(Some((row, false)));
        policies.push(Some((row, true)));
    }
    let mut claims = vec![ReachClaim::Slotless, PRIVATE, OWN, UNREADABLE];
    claims.extend((3..=8).map(|openness| ReachClaim::Declared(DeclaredReach::Wider(openness))));
    let mut permitted = 0;
    for policy in &policies {
        for who in [Relationship::None, Relationship::OwnDevice] {
            for claim in &claims {
                for object in [ObjectKind::ManifestOrChunk, ObjectKind::MethodOrInput] {
                    // The serve rows of the truth table, stated independently of the function.
                    let expected = match (policy, who, claim) {
                        (Some((row, disclose)), Relationship::OwnDevice, ReachClaim::Slotless) => {
                            (row.grants() && (object == ObjectKind::ManifestOrChunk || *disclose))
                                .then_some(Basis::MethodRow)
                        }
                        (Some((row, _)), Relationship::OwnDevice, claim) if *claim == OWN => {
                            (!row.narrows()).then_some(Basis::Reach)
                        }
                        _ => None,
                    };
                    let outcome = decide(*policy, who, *claim, object);
                    assert_eq!(
                        outcome.ok(),
                        expected,
                        "{policy:?} {who:?} {claim:?} {object:?} -> {outcome:?}"
                    );
                    permitted += usize::from(expected.is_some());
                }
            }
        }
    }
    // Slotless: the two rows naming the steward serve bytes under either disclose value and a
    // method object only when disclosed (2 x 3); self: the four rows that do not narrow, both
    // disclose values, both objects (4 x 2 x 2).
    assert_eq!(permitted, 2 * 3 + 4 * 2 * 2);
}

#[test]
fn an_unreadable_reach_slot_is_private() {
    for value in [
        "", "Self", "SELF", "PUBLIC", "selff", " self", "self ", "own", "steward",
    ] {
        assert_eq!(parse_reach(value), DeclaredReach::Unreadable, "{value:?}");
    }
    let (output, manifest) = blobs();
    let twice = custody(
        &output,
        &manifest,
        ME,
        &["env:reach=self", "env:reach=self"],
    );
    assert_eq!(
        event_reach(&twice),
        Some(DeclaredReach::Unreadable),
        "two slots in one event"
    );
    for event in [twice, custody(&output, &manifest, ME, &["env:reach=Self"])] {
        let claim = claim_of(std::slice::from_ref(&event), &manifest).expect("own custody");
        assert_eq!(claim, UNREADABLE);
        let outcome = decide(
            Some((Row::Steward, true)),
            Relationship::OwnDevice,
            claim,
            ObjectKind::ManifestOrChunk,
        );
        assert_eq!(outcome, Err(Deny::Private));
    }
    assert_eq!(event_reach(&custody(&output, &manifest, ME, &[])), None);

    // A near-miss key is a reach slot the gate cannot read, never an absent one: a mistyped
    // `private` must not fall back to the method-row path.
    for slot in [
        "env:reach",
        "env:reach:private",
        "env:Reach=private",
        "ENV:REACH=self",
        "env:reach =self",
        "env:reachable=self",
    ] {
        let event = custody(&output, &manifest, ME, &[slot]);
        assert_eq!(
            event_reach(&event),
            Some(DeclaredReach::Unreadable),
            "{slot}"
        );
        let claim = claim_of(std::slice::from_ref(&event), &manifest).expect("own custody");
        assert_eq!(claim, UNREADABLE, "{slot}");
    }
    // A well-formed slot beside a malformed one is still two reach slots in one event.
    let mixed = custody(
        &output,
        &manifest,
        ME,
        &["env:reach=self", "env:Reach=self"],
    );
    assert_eq!(event_reach(&mixed), Some(DeclaredReach::Unreadable));
    // Other env slots are not reach slots.
    let other = custody(&output, &manifest, ME, &["env:requires=bafkreiexample"]);
    assert_eq!(event_reach(&other), None);
}

#[test]
fn an_unavailable_policy_is_never_an_empty_list() {
    let empty = ArtifactServeList::default();
    let decide_under = |policy: ServePolicy<'_>| {
        serve_decision(
            Relationship::OwnDevice,
            &OWN,
            METHOD,
            ObjectKind::ManifestOrChunk,
            &policy,
        )
    };
    assert_eq!(
        decide_under(ServePolicy::from(None)),
        Err(Deny::PolicyUnavailable)
    );
    // The hazard T5 names: a defaulted list narrows nothing, so it would serve a `self` record.
    // Only a policy that parsed may be passed as loaded.
    assert_eq!(
        decide_under(ServePolicy::from(Some(&empty))),
        Ok(Basis::Reach)
    );
}

#[test]
fn the_narrowest_slot_of_a_manifest_decides() {
    let (output, manifest) = blobs();
    let slotted = |value: &str| custody(&output, &manifest, ME, &[&format!("env:reach={value}")]);
    let fold = |values: &[&str]| {
        let events: Vec<_> = values.iter().map(|value| slotted(value)).collect();
        claim_of(&events, &manifest).expect("own custody")
    };
    assert_eq!(fold(&["self", "private"]), PRIVATE);
    assert_eq!(fold(&["private", "self"]), PRIVATE, "order does not matter");
    assert_eq!(fold(&["public", "self"]), OWN);
    assert_eq!(fold(&["self", "public"]), OWN);
    assert_eq!(
        fold(&["commons", "community"]),
        ReachClaim::Declared(DeclaredReach::Wider(6))
    );
    assert_eq!(
        fold(&["self", "nonsense"]),
        UNREADABLE,
        "unreadable is narrower than private"
    );
    assert_eq!(fold(&["private", "nonsense", "self"]), UNREADABLE);
}

#[test]
fn a_slotless_reput_cannot_undo_a_slot() {
    let (output, manifest) = blobs();
    let slotless = custody(&output, &manifest, ME, &[]);
    for value in ["self", "private", "public"] {
        let slotted = custody(&output, &manifest, ME, &[&format!("env:reach={value}")]);
        let events = [slotted, slotless.clone(), slotless.clone()];
        let claim = claim_of(&events, &manifest).expect("own custody");
        assert_eq!(claim, ReachClaim::Declared(parse_reach(value)), "{value}");
    }
    assert_eq!(
        claim_of(&[slotless.clone(), slotless], &manifest),
        Some(ReachClaim::Slotless)
    );
}

#[test]
fn a_sibling_or_foreign_custodian_event_contributes_no_claim() {
    let (output, manifest) = blobs();
    let folded = custody(
        &output,
        &manifest,
        ME,
        &["env:reach=self", &format!("via:{SIBLING}")],
    );
    let foreign = custody(&output, &manifest, SIBLING, &["env:reach=self"]);
    assert_eq!(
        claim_of(&[folded.clone(), foreign.clone()], &manifest),
        None
    );

    // Beside this device's own records, they change nothing in either direction.
    let own_slotless = custody(&output, &manifest, ME, &[]);
    let events = [own_slotless, folded.clone(), foreign.clone()];
    assert_eq!(claim_of(&events, &manifest), Some(ReachClaim::Slotless));
    let own_private = custody(&output, &manifest, ME, &["env:reach=private"]);
    let events = [own_private, folded, foreign];
    assert_eq!(claim_of(&events, &manifest), Some(PRIVATE));

    // A Produce that is not custody of a copy here names no manifest.
    let mut not_custody = custody(&output, &manifest, ME, &["env:reach=self"]);
    not_custody.action = ReaVerb::Use;
    assert_eq!(claim_of(&[not_custody], &manifest), None);
}

#[test]
fn the_gates_reach_names_and_openness_match_elohim_epr() {
    use elohim_epr::Reach;
    // The exhaustive match pins the variant list: a new `Reach` variant breaks this test.
    let all = [
        Reach::Private,
        Reach::SelfScope,
        Reach::Intimate,
        Reach::Trusted,
        Reach::Familiar,
        Reach::Community,
        Reach::Public,
        Reach::Commons,
    ];
    for reach in all {
        match reach {
            Reach::Private
            | Reach::SelfScope
            | Reach::Intimate
            | Reach::Trusted
            | Reach::Familiar
            | Reach::Community
            | Reach::Public
            | Reach::Commons => {}
        }
    }
    let names: Vec<(String, u8)> = all
        .iter()
        .map(|reach| {
            let name = serde_json::to_value(reach).expect("serialize");
            (
                name.as_str().expect("a string").to_owned(),
                reach.openness(),
            )
        })
        .collect();
    let gate: Vec<(String, u8)> = REACH_LEVELS
        .iter()
        .map(|(name, openness)| ((*name).to_owned(), *openness))
        .collect();
    assert_eq!(gate, names);
    for (name, openness) in REACH_LEVELS {
        let expected = match openness {
            1 => DeclaredReach::Private,
            2 => DeclaredReach::OwnDevices,
            wider => DeclaredReach::Wider(wider),
        };
        assert_eq!(parse_reach(name), expected, "{name}");
    }
}
