//! `epr flow intend` — author the local and crossing records of the dev landing queue.
//!
//! An intent atom is stable queue identity: only the change, trees, class, closures and optional
//! predecessor participate in its CID.  Device time and ordinal belong to the accompanying
//! `raise` event.  Verification copies that local event into the crossing `intent` event; landing,
//! withdrawal and dismissal are later observations of the same resource.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{SecondsFormat, Utc};
use cid::Cid;
use elohim_epr_rea::{
    AgentRef, FlowEvent, FlowRecord, FlowStore, Intent, Magnitude, ReaVerb, ResourceSpec,
    SidecarFlowStore,
};
use serde::Serialize;

use super::measures::{self, MeasureRef, Registry};
use super::note::{
    named_identity, resolve_attribution, NoteActor, ENV_SLOT_PREFIX, MEASURE_SLOT_PREFIX,
    REASON_SLOT_PREFIX, UNIT_SLOT_PREFIX, VALUE_SLOT_PREFIX,
};
use super::{
    head_commit_provenance, repository_agent, repository_scope, short_cid, FlowError, FlowResult,
};

pub(crate) const LANDING_MEASURE: &str = "dev-landing@1";
pub(crate) const INTENT_TAG: &str = "dev-landing:intent";
pub(crate) const KINDS: [&str; 4] = ["intent", "landed", "withdraw", "dismiss"];
pub(crate) const LANDING_UNIT: &str = "dev-landing";
pub(crate) const CLASSES_LITERAL: [&str; 5] = ["0", "low", "channel", "governance", "high"];
pub(crate) const CLASSES: [&str; 5] = CLASSES_LITERAL;

const TREE_PREFIX: &str = "tree:";
const BASE_PREFIX: &str = "base:";
const CLASS_PREFIX: &str = "class:";
const CLOSURE_PREFIX: &str = "closure:";
const SUPERSEDES_PREFIX: &str = "supersedes:";
const VIA_PREFIX: &str = "via:";

/// One of the five mutually exclusive authoring acts.
pub enum IntendMode<'a> {
    Raise(RaiseRequest<'a>),
    Verified(VerifiedRequest<'a>),
    Land(LandRequest<'a>),
    Withdraw(WithdrawRequest<'a>),
    Dismiss(DismissRequest<'a>),
}

pub struct RaiseRequest<'a> {
    pub change: &'a str,
    pub tree: &'a str,
    pub base: &'a str,
    pub class: &'a str,
    pub closures: &'a [String],
    pub predicted_base: Option<&'a str>,
    pub chain_len: Option<u64>,
    pub supersedes: Option<&'a str>,
    pub session: Option<&'a str>,
}

pub struct VerifiedRequest<'a> {
    pub intent: &'a str,
    pub gate_s: Option<u64>,
}

pub struct LandRequest<'a> {
    pub intent: &'a str,
    pub tip: &'a str,
    pub commit: &'a str,
    pub landed_as: &'a str,
    pub regated_s: Option<u64>,
}

pub struct WithdrawRequest<'a> {
    pub intent: &'a str,
    pub why: Option<&'a str>,
}

pub struct DismissRequest<'a> {
    pub intent: &'a str,
    pub capsule: &'a [String],
}

pub struct IntendRequest<'a> {
    pub mode: IntendMode<'a>,
    pub measures: Option<&'a Path>,
    pub actor: &'a NoteActor,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntendOutcome {
    pub mode: &'static str,
    pub intent: String,
    pub change: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    pub dseq: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    pub store: String,
    pub appended: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub superseded: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

impl IntendOutcome {
    pub fn render(&self) {
        println!(
            "intend  {:<9} {}  change {}  dseq {}{}",
            self.mode,
            short_cid_str(&self.intent),
            self.change,
            self.dseq,
            if self.appended { "" } else { "  (no-op)" }
        );
        if let Some(event) = &self.event {
            println!("        event: {}", short_cid_str(event));
        }
        if let Some(superseded) = &self.superseded {
            println!("        supersedes: {}", short_cid_str(superseded));
        }
        if let Some(state) = &self.state {
            println!("        state: {state}");
        }
    }
}

fn short_cid_str(raw: &str) -> String {
    raw.parse::<Cid>()
        .map_or_else(|_| raw.to_string(), |cid| short_cid(&cid))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OwnState {
    Live,
    Landed,
    Withdrawn,
    Dismissed,
    Superseded,
}

impl OwnState {
    fn word(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Landed => "landed",
            Self::Withdrawn => "withdrawn",
            Self::Dismissed => "dismissed",
            Self::Superseded => "superseded",
        }
    }
}

/// Fold only this store's own events and successor atoms for one local intent.
pub(crate) fn own_state(records: &[(Cid, FlowRecord)], intent: &Cid) -> OwnState {
    let mut state = OwnState::Live;
    let intent_text = intent.to_string();
    for (_, record) in records {
        match record {
            FlowRecord::Intent(atom)
                if atom.resource_spec.classified_as.first().map(String::as_str)
                    == Some(INTENT_TAG)
                    && slot(&atom.resource_spec.classified_as, SUPERSEDES_PREFIX)
                        == Some(intent_text.as_str()) =>
            {
                if state == OwnState::Live {
                    state = OwnState::Superseded;
                }
            }
            FlowRecord::Event(event)
                if event.resource == *intent
                    && !event
                        .classified_as
                        .iter()
                        .any(|value| value.starts_with(VIA_PREFIX))
                    && slot(&event.classified_as, MEASURE_SLOT_PREFIX) == Some(LANDING_MEASURE) =>
            {
                match slot(&event.classified_as, REASON_SLOT_PREFIX) {
                    Some("landed") => state = OwnState::Landed,
                    Some("withdraw") if state != OwnState::Landed => {
                        state = OwnState::Withdrawn;
                    }
                    Some("dismiss") if state != OwnState::Landed => state = OwnState::Dismissed,
                    _ => {}
                }
            }
            _ => {}
        }
    }
    state
}

/// Resolve and append exactly one landing-queue act.
pub fn intend(root: &Path, request: &IntendRequest<'_>) -> FlowResult<IntendOutcome> {
    served_store_guard(root)?;
    validate_request(&request.mode)?;
    let registry_path = request
        .measures
        .map(Path::to_path_buf)
        .unwrap_or_else(|| measures::default_measures(root));
    let measure = MeasureRef::parse(LANDING_MEASURE, &registry_path)?;
    let registry = Registry::open(&registry_path)?;
    if !registry.declares(&measure) {
        return Err(FlowError::InvalidArguments(format!(
            "unknown measure `{LANDING_MEASURE}` — declare it in {} before writing a landing record",
            registry_path.display()
        )));
    }
    let unit_slot = registry.declared_unit(&measure);
    let named = named_identity(request.actor.as_ref.as_deref())?;
    let (author, occurred_at) = head_commit_provenance(root).unwrap_or_else(|| {
        (
            repository_agent(root).map_or_else(|_| "unknown".to_string(), |agent| agent.0),
            Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        )
    });
    let attribution = resolve_attribution(root, named, None, &author);
    let provider = attribution.provider(&author);
    let receiver = repository_agent(root)?;
    let scope = repository_scope(root)?;

    if !matches!(&request.mode, IntendMode::Raise(_))
        && !root.join(".eprfs/status/flows.jsonl").is_file()
    {
        return Err(FlowError::UnknownResource(
            "no flow store exists yet; raise the intent before settling or verifying it"
                .to_string(),
        ));
    }

    let mut store = SidecarFlowStore::open(root)?.transaction()?;
    let records = store.records()?;
    let common = EventCommon {
        provider: &provider,
        receiver: &receiver,
        scope: &scope,
        occurred_at: &occurred_at,
        unit_slot: unit_slot.as_deref(),
        attribution: &attribution,
    };
    match &request.mode {
        IntendMode::Raise(req) => raise(root, req, &records, &mut store, &common),
        IntendMode::Verified(req) => verified(root, req, &records, &mut store, &common),
        IntendMode::Land(req) => land(root, req, &records, &mut store, &common),
        IntendMode::Withdraw(req) => withdraw(root, req, &records, &mut store, &common),
        IntendMode::Dismiss(req) => dismiss(root, req, &records, &mut store, &common),
    }
}

fn validate_request(mode: &IntendMode<'_>) -> FlowResult<()> {
    match mode {
        IntendMode::Raise(request) => {
            validate_change(request.change)?;
            validate_oid(request.tree, "--tree")?;
            validate_oid(request.base, "--base")?;
            if !CLASSES.contains(&request.class) {
                return Err(FlowError::InvalidArguments(format!(
                    "--class needs one of {}, got `{}`",
                    CLASSES.join("|"),
                    request.class
                )));
            }
            parse_pairs(request.closures, "--closure", PairRule::HexValue)?;
            if let Some(predicted) = request.predicted_base.filter(|value| *value != "dev") {
                validate_oid(predicted, "--predicted-base")?;
            }
            if let Some(cid) = request.supersedes {
                parse_cid(cid)?;
            }
        }
        IntendMode::Verified(request) => {
            parse_cid(request.intent)?;
        }
        IntendMode::Land(request) => {
            parse_cid(request.intent)?;
            validate_oid(request.tip, "--tip")?;
            validate_oid(request.commit, "--commit")?;
            if !matches!(request.landed_as, "ff" | "moved") {
                return Err(FlowError::InvalidArguments(
                    "--landed-as needs ff|moved".to_string(),
                ));
            }
        }
        IntendMode::Withdraw(request) => {
            parse_cid(request.intent)?;
            if request.why.is_some_and(|why| why.contains(['\n', '\r'])) {
                return Err(FlowError::InvalidArguments(
                    "--why must be one line".to_string(),
                ));
            }
        }
        IntendMode::Dismiss(request) => {
            parse_cid(request.intent)?;
            let capsule = parse_pairs(request.capsule, "--capsule", PairRule::Capsule)?;
            validate_capsule_keys(&capsule)?;
        }
    }
    Ok(())
}

struct EventCommon<'a> {
    provider: &'a str,
    receiver: &'a AgentRef,
    scope: &'a Cid,
    occurred_at: &'a str,
    unit_slot: Option<&'a str>,
    attribution: &'a super::note::Attribution,
}

fn raise(
    root: &Path,
    request: &RaiseRequest<'_>,
    records: &[(Cid, FlowRecord)],
    store: &mut impl FlowStore,
    common: &EventCommon<'_>,
) -> FlowResult<IntendOutcome> {
    validate_change(request.change)?;
    validate_oid(request.tree, "--tree")?;
    validate_oid(request.base, "--base")?;
    if !CLASSES.contains(&request.class) {
        return Err(FlowError::InvalidArguments(format!(
            "--class needs one of {}, got `{}`",
            CLASSES.join("|"),
            request.class
        )));
    }
    let closures = parse_pairs(request.closures, "--closure", PairRule::HexValue)?;
    let predicted_base = request.predicted_base.unwrap_or("dev");
    if predicted_base != "dev" {
        validate_oid(predicted_base, "--predicted-base")?;
    }
    let supersedes = request.supersedes.map(parse_cid).transpose()?;
    if let Some(prior) = supersedes.as_ref() {
        let prior_data = own_atom(records, prior).ok_or_else(|| {
            FlowError::UnknownResource(format!(
                "{prior} (--supersedes must name an own {INTENT_TAG} atom)"
            ))
        })?;
        if prior_data.change != request.change {
            return Err(FlowError::InvalidArguments(format!(
                "--supersedes {prior} belongs to {}, not {}",
                prior_data.change, request.change
            )));
        }
    }

    let desired = IntentData {
        change: request.change.to_string(),
        tree: request.tree.to_string(),
        base: request.base.to_string(),
        class: request.class.to_string(),
        closures,
        supersedes: supersedes.as_ref().map(ToString::to_string),
    };
    let atoms = own_atoms(records);
    let matching: Vec<_> = atoms
        .iter()
        .filter(|(_, data)| data.semantic_eq(&desired))
        .collect();
    if let Some((cid, data)) = matching
        .iter()
        .find(|(cid, _)| own_state(records, cid) == OwnState::Live)
        .copied()
    {
        if let Some(event) = raise_event(records, cid) {
            return Ok(outcome_from_event(OutcomeEvent {
                mode: "raise",
                root,
                intent: cid,
                change: &data.change,
                event,
                appended: false,
                superseded: data.supersedes.clone(),
                state: Some(OwnState::Live.word().to_string()),
            }));
        }
        return append_raise_event(
            root,
            request,
            records,
            store,
            common,
            *cid,
            data.supersedes.clone(),
        );
    }
    if let Some((cid, data)) = matching
        .iter()
        .find(|(cid, _)| own_state(records, cid) == OwnState::Landed)
        .copied()
    {
        return if let Some(event) = raise_event(records, cid) {
            Ok(outcome_from_event(OutcomeEvent {
                mode: "raise",
                root,
                intent: cid,
                change: &data.change,
                event,
                appended: false,
                superseded: data.supersedes.clone(),
                state: Some(OwnState::Landed.word().to_string()),
            }))
        } else {
            Ok(IntendOutcome {
                mode: "raise",
                intent: cid.to_string(),
                change: data.change.clone(),
                event: None,
                dseq: 0,
                at: None,
                store: store_name(root),
                appended: false,
                superseded: data.supersedes.clone(),
                state: Some(OwnState::Landed.word().to_string()),
            })
        };
    }

    let live_for_change = atoms.iter().find(|(cid, data)| {
        data.change == request.change && own_state(records, cid) == OwnState::Live
    });
    if let Some((incumbent, _)) = live_for_change {
        if supersedes.as_ref() != Some(incumbent) {
            return Err(FlowError::InvalidArguments(format!(
                "change {} already has live intent {incumbent}; pass --supersedes {incumbent} to replace it",
                request.change
            )));
        }
    }

    let implicit_predecessor = matching.iter().rev().find_map(|(cid, _)| {
        let dead = matches!(
            own_state(records, cid),
            OwnState::Withdrawn | OwnState::Dismissed | OwnState::Superseded
        );
        let has_successor = atoms
            .iter()
            .any(|(_, atom)| atom.supersedes.as_deref() == Some(&cid.to_string()));
        (dead && !has_successor).then_some(*cid)
    });
    let predecessor = supersedes.or(implicit_predecessor);
    let mut atom_slots = vec![
        INTENT_TAG.to_string(),
        request.change.to_string(),
        format!("{TREE_PREFIX}{}", request.tree),
        format!("{BASE_PREFIX}{}", request.base),
        format!("{CLASS_PREFIX}{}", request.class),
    ];
    for (project, closure) in &desired.closures {
        atom_slots.push(format!("{CLOSURE_PREFIX}{project}={closure}"));
    }
    if let Some(prior) = predecessor.as_ref() {
        atom_slots.push(format!("{SUPERSEDES_PREFIX}{prior}"));
    }
    common.attribution.append_slots(&mut atom_slots);
    let atom = FlowRecord::Intent(Intent {
        action: ReaVerb::Produce,
        resource_spec: ResourceSpec {
            quantity: None,
            classified_as: atom_slots,
        },
        in_scope_of: *common.scope,
        raised_by: AgentRef(common.provider.to_string()),
    });
    let intent = atom.cid()?;
    store.append(atom)?;
    append_raise_event(
        root,
        request,
        records,
        store,
        common,
        intent,
        predecessor.map(|cid| cid.to_string()),
    )
}

fn append_raise_event(
    root: &Path,
    request: &RaiseRequest<'_>,
    records: &[(Cid, FlowRecord)],
    store: &mut impl FlowStore,
    common: &EventCommon<'_>,
    intent: Cid,
    predecessor: Option<String>,
) -> FlowResult<IntendOutcome> {
    let dseq = next_dseq(records);
    let at = unix_seconds_millis();
    let predicted_base = request.predicted_base.unwrap_or("dev");
    let mut env = BTreeMap::from([
        ("at".to_string(), at.clone()),
        (
            "chain_len".to_string(),
            request.chain_len.unwrap_or(0).to_string(),
        ),
        ("change".to_string(), request.change.to_string()),
        ("dseq".to_string(), dseq.to_string()),
        ("predicted_base".to_string(), predicted_base.to_string()),
        ("tree".to_string(), request.tree.to_string()),
    ]);
    if let Some(session) = request.session.filter(|value| !value.is_empty()) {
        env.insert("session".to_string(), session.to_string());
    }
    let event = event_record("raise", ReaVerb::Cite, intent, vec![], env, common);
    let event_cid = event.cid()?;
    store.append(event)?;
    Ok(IntendOutcome {
        mode: "raise",
        intent: intent.to_string(),
        change: request.change.to_string(),
        event: Some(event_cid.to_string()),
        dseq,
        at: Some(at),
        store: store_name(root),
        appended: true,
        superseded: predecessor,
        state: Some("live".to_string()),
    })
}

fn verified(
    root: &Path,
    request: &VerifiedRequest<'_>,
    records: &[(Cid, FlowRecord)],
    store: &mut impl FlowStore,
    common: &EventCommon<'_>,
) -> FlowResult<IntendOutcome> {
    let intent = parse_cid(request.intent)?;
    let data = own_atom(records, &intent).ok_or_else(|| {
        FlowError::UnknownResource(format!(
            "{} (verified must name an own intent atom)",
            request.intent
        ))
    })?;
    if let Some(existing) = own_event(records, &intent, "intent") {
        return Ok(outcome_from_event(OutcomeEvent {
            mode: "verified",
            root,
            intent: &intent,
            change: &data.change,
            event: existing,
            appended: false,
            superseded: data.supersedes,
            state: None,
        }));
    }
    let raise = raise_event(records, &intent).ok_or_else(|| {
        FlowError::InvalidArguments(format!(
            "intent {intent} has no local raise event; re-raise the intent before verifying it"
        ))
    })?;
    let raise_env = event_env(raise);
    let mut env = BTreeMap::new();
    for key in ["at", "chain_len", "dseq", "predicted_base", "session"] {
        if let Some(value) = raise_env.get(key) {
            env.insert(key.to_string(), (*value).to_string());
        }
    }
    env.insert("base".to_string(), data.base.clone());
    env.insert("change".to_string(), data.change.clone());
    env.insert("class".to_string(), data.class.clone());
    env.insert("closure".to_string(), joined_closures(&data.closures));
    env.insert("tree".to_string(), data.tree.clone());
    env.insert("verified_at".to_string(), unix_seconds_millis());
    if let Some(prior) = &data.supersedes {
        env.insert("supersedes".to_string(), prior.clone());
    }
    if let Some(gate_s) = request.gate_s {
        env.insert("gate_s".to_string(), gate_s.to_string());
    }
    let event = event_record("intent", ReaVerb::Cite, intent, vec![], env, common);
    let event_cid = event.cid()?;
    let parsed = event_as_ref(&event)?;
    let outcome = outcome_from_event(OutcomeEvent {
        mode: "verified",
        root,
        intent: &intent,
        change: &data.change,
        event: parsed,
        appended: true,
        superseded: data.supersedes,
        state: None,
    });
    store.append(event)?;
    Ok(IntendOutcome {
        event: Some(event_cid.to_string()),
        ..outcome
    })
}

fn land(
    root: &Path,
    request: &LandRequest<'_>,
    records: &[(Cid, FlowRecord)],
    store: &mut impl FlowStore,
    common: &EventCommon<'_>,
) -> FlowResult<IntendOutcome> {
    validate_oid(request.tip, "--tip")?;
    validate_oid(request.commit, "--commit")?;
    if !matches!(request.landed_as, "ff" | "moved") {
        return Err(FlowError::InvalidArguments(
            "--landed-as needs ff|moved".to_string(),
        ));
    }
    let (intent, data) = target(records, request.intent)?;
    let mut env = settlement_env(records, &data.change);
    env.insert("commit".to_string(), request.commit.to_string());
    env.insert("landed_as".to_string(), request.landed_as.to_string());
    env.insert("tip".to_string(), request.tip.to_string());
    env.insert("tree".to_string(), data.tree.clone());
    if let Some(value) = request.regated_s {
        env.insert("regated_s".to_string(), value.to_string());
    }
    settle(
        root,
        Settlement {
            mode: "land",
            kind: "landed",
            action: ReaVerb::Produce,
            intent,
            data,
            satisfies: vec![intent],
            env,
        },
        records,
        store,
        common,
    )
}

fn withdraw(
    root: &Path,
    request: &WithdrawRequest<'_>,
    records: &[(Cid, FlowRecord)],
    store: &mut impl FlowStore,
    common: &EventCommon<'_>,
) -> FlowResult<IntendOutcome> {
    let (intent, data) = target(records, request.intent)?;
    let mut env = settlement_env(records, &data.change);
    if let Some(why) = request.why {
        if why.contains(['\n', '\r']) {
            return Err(FlowError::InvalidArguments(
                "--why must be one line".to_string(),
            ));
        }
        env.insert("why".to_string(), why.to_string());
    }
    settle(
        root,
        Settlement {
            mode: "withdraw",
            kind: "withdraw",
            action: ReaVerb::Cite,
            intent,
            data,
            satisfies: vec![],
            env,
        },
        records,
        store,
        common,
    )
}

fn dismiss(
    root: &Path,
    request: &DismissRequest<'_>,
    records: &[(Cid, FlowRecord)],
    store: &mut impl FlowStore,
    common: &EventCommon<'_>,
) -> FlowResult<IntendOutcome> {
    let (intent, data) = target(records, request.intent)?;
    let capsule = parse_pairs(request.capsule, "--capsule", PairRule::Capsule)?;
    validate_capsule_keys(&capsule)?;
    let mut env = settlement_env(records, &data.change);
    env.extend(capsule);
    settle(
        root,
        Settlement {
            mode: "dismiss",
            kind: "dismiss",
            action: ReaVerb::Dismiss,
            intent,
            data,
            satisfies: vec![],
            env,
        },
        records,
        store,
        common,
    )
}

fn validate_capsule_keys(capsule: &BTreeMap<String, String>) -> FlowResult<()> {
    let required: BTreeSet<&str> = [
        "key_predicted",
        "key_dev",
        "project",
        "chain",
        "heads",
        "now",
    ]
    .into_iter()
    .collect();
    let actual: BTreeSet<&str> = capsule.keys().map(String::as_str).collect();
    if actual != required {
        return Err(FlowError::InvalidArguments(format!(
            "--dismiss needs exactly one --capsule for each of {}",
            required.into_iter().collect::<Vec<_>>().join(", ")
        )));
    }
    Ok(())
}

struct Settlement {
    mode: &'static str,
    kind: &'static str,
    action: ReaVerb,
    intent: Cid,
    data: IntentData,
    satisfies: Vec<Cid>,
    env: BTreeMap<String, String>,
}

fn settle(
    root: &Path,
    settlement: Settlement,
    records: &[(Cid, FlowRecord)],
    store: &mut impl FlowStore,
    common: &EventCommon<'_>,
) -> FlowResult<IntendOutcome> {
    if let Some(existing) = own_event(records, &settlement.intent, settlement.kind) {
        return Ok(outcome_from_event(OutcomeEvent {
            mode: settlement.mode,
            root,
            intent: &settlement.intent,
            change: &settlement.data.change,
            event: existing,
            appended: false,
            superseded: settlement.data.supersedes,
            state: None,
        }));
    }
    let event = event_record(
        settlement.kind,
        settlement.action,
        settlement.intent,
        settlement.satisfies,
        settlement.env,
        common,
    );
    let event_cid = event.cid()?;
    let parsed = event_as_ref(&event)?;
    let outcome = outcome_from_event(OutcomeEvent {
        mode: settlement.mode,
        root,
        intent: &settlement.intent,
        change: &settlement.data.change,
        event: parsed,
        appended: true,
        superseded: settlement.data.supersedes,
        state: None,
    });
    store.append(event)?;
    Ok(IntendOutcome {
        event: Some(event_cid.to_string()),
        ..outcome
    })
}

fn event_record(
    kind: &str,
    action: ReaVerb,
    resource: Cid,
    satisfies: Vec<Cid>,
    env: BTreeMap<String, String>,
    common: &EventCommon<'_>,
) -> FlowRecord {
    let change = env.get("change").cloned().unwrap_or_default();
    let mut classified_as = vec![
        format!("dev-landing:{kind}"),
        change,
        format!("{REASON_SLOT_PREFIX}{kind}"),
        format!("{MEASURE_SLOT_PREFIX}{LANDING_MEASURE}"),
        format!("{VALUE_SLOT_PREFIX}1"),
    ];
    if let Some(unit) = common.unit_slot {
        classified_as.push(format!("{UNIT_SLOT_PREFIX}{unit}"));
    }
    for (key, value) in env {
        classified_as.push(format!("{ENV_SLOT_PREFIX}{key}={value}"));
    }
    common.attribution.append_slots(&mut classified_as);
    FlowRecord::Event(FlowEvent {
        action,
        provider: AgentRef(common.provider.to_string()),
        receiver: common.receiver.clone(),
        resource,
        quantity: Magnitude::Count {
            value: 1.0,
            unit: LANDING_UNIT.to_string(),
        },
        process: None,
        in_scope_of: *common.scope,
        fulfills: Vec::new(),
        satisfies,
        occurred_at: common.occurred_at.to_string(),
        classified_as,
    })
}

#[derive(Clone)]
struct IntentData {
    change: String,
    tree: String,
    base: String,
    class: String,
    closures: BTreeMap<String, String>,
    supersedes: Option<String>,
}

impl IntentData {
    fn semantic_eq(&self, other: &Self) -> bool {
        self.change == other.change
            && self.tree == other.tree
            && self.base == other.base
            && self.class == other.class
            && self.closures == other.closures
    }
}

fn own_atoms(records: &[(Cid, FlowRecord)]) -> Vec<(Cid, IntentData)> {
    records
        .iter()
        .filter_map(|(cid, record)| match record {
            FlowRecord::Intent(atom)
                if atom.resource_spec.classified_as.first().map(String::as_str)
                    == Some(INTENT_TAG) =>
            {
                parse_atom(&atom.resource_spec.classified_as).map(|data| (*cid, data))
            }
            _ => None,
        })
        .collect()
}

fn own_atom(records: &[(Cid, FlowRecord)], cid: &Cid) -> Option<IntentData> {
    own_atoms(records)
        .into_iter()
        .find_map(|(found, data)| (found == *cid).then_some(data))
}

fn parse_atom(slots: &[String]) -> Option<IntentData> {
    Some(IntentData {
        change: slots.get(1)?.clone(),
        tree: slot(slots, TREE_PREFIX)?.to_string(),
        base: slot(slots, BASE_PREFIX)?.to_string(),
        class: slot(slots, CLASS_PREFIX)?.to_string(),
        closures: slots
            .iter()
            .filter_map(|value| value.strip_prefix(CLOSURE_PREFIX)?.split_once('='))
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect(),
        supersedes: slot(slots, SUPERSEDES_PREFIX).map(str::to_string),
    })
}

fn target(records: &[(Cid, FlowRecord)], raw: &str) -> FlowResult<(Cid, IntentData)> {
    let cid = parse_cid(raw)?;
    if let Some(data) = own_atom(records, &cid) {
        return Ok((cid, data));
    }
    for (_, record) in records {
        let FlowRecord::Event(event) = record else {
            continue;
        };
        if event.resource != cid
            || slot(&event.classified_as, MEASURE_SLOT_PREFIX) != Some(LANDING_MEASURE)
            || slot(&event.classified_as, REASON_SLOT_PREFIX) != Some("intent")
        {
            continue;
        }
        let env = event_env(event);
        let data = IntentData {
            change: env.get("change").copied().unwrap_or_default().to_string(),
            tree: env.get("tree").copied().unwrap_or_default().to_string(),
            base: env.get("base").copied().unwrap_or_default().to_string(),
            class: env.get("class").copied().unwrap_or_default().to_string(),
            closures: parse_joined_closures(env.get("closure").copied().unwrap_or_default()),
            supersedes: env.get("supersedes").map(|value| (*value).to_string()),
        };
        return Ok((cid, data));
    }
    Err(FlowError::UnknownResource(format!(
        "{raw} (not an own intent atom or a folded landing-intent announcement)"
    )))
}

fn own_event<'a>(
    records: &'a [(Cid, FlowRecord)],
    intent: &Cid,
    kind: &str,
) -> Option<&'a FlowEvent> {
    records.iter().find_map(|(_, record)| match record {
        FlowRecord::Event(event)
            if event.resource == *intent
                && !event
                    .classified_as
                    .iter()
                    .any(|value| value.starts_with(VIA_PREFIX))
                && slot(&event.classified_as, MEASURE_SLOT_PREFIX) == Some(LANDING_MEASURE)
                && slot(&event.classified_as, REASON_SLOT_PREFIX) == Some(kind) =>
        {
            Some(event)
        }
        _ => None,
    })
}

fn raise_event<'a>(records: &'a [(Cid, FlowRecord)], intent: &Cid) -> Option<&'a FlowEvent> {
    own_event(records, intent, "raise")
}

fn event_env(event: &FlowEvent) -> BTreeMap<&str, &str> {
    event
        .classified_as
        .iter()
        .filter_map(|value| value.strip_prefix(ENV_SLOT_PREFIX)?.split_once('='))
        .collect()
}

fn event_as_ref(record: &FlowRecord) -> FlowResult<&FlowEvent> {
    match record {
        FlowRecord::Event(event) => Ok(event),
        _ => Err(FlowError::InvalidArguments(
            "internal landing record is not an event".to_string(),
        )),
    }
}

struct OutcomeEvent<'a> {
    mode: &'static str,
    root: &'a Path,
    intent: &'a Cid,
    change: &'a str,
    event: &'a FlowEvent,
    appended: bool,
    superseded: Option<String>,
    state: Option<String>,
}

fn outcome_from_event(input: OutcomeEvent<'_>) -> IntendOutcome {
    let env = event_env(input.event);
    IntendOutcome {
        mode: input.mode,
        intent: input.intent.to_string(),
        change: input.change.to_string(),
        event: input.event.cid().ok().map(|cid| cid.to_string()),
        dseq: env
            .get("dseq")
            .and_then(|raw| raw.parse().ok())
            .unwrap_or(0),
        at: env.get("at").map(|value| (*value).to_string()),
        store: store_name(input.root),
        appended: input.appended,
        superseded: input.superseded,
        state: input.state,
    }
}

trait EventCid {
    fn cid(&self) -> Result<Cid, elohim_epr_rea::FabricError>;
}

impl EventCid for FlowEvent {
    fn cid(&self) -> Result<Cid, elohim_epr_rea::FabricError> {
        FlowRecord::Event(self.clone()).cid()
    }
}

fn next_dseq(records: &[(Cid, FlowRecord)]) -> u64 {
    records
        .iter()
        .filter_map(|(_, record)| match record {
            FlowRecord::Event(event)
                if !event
                    .classified_as
                    .iter()
                    .any(|value| value.starts_with(VIA_PREFIX))
                    && slot(&event.classified_as, MEASURE_SLOT_PREFIX) == Some(LANDING_MEASURE) =>
            {
                event_env(event).get("dseq")?.parse::<u64>().ok()
            }
            _ => None,
        })
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

fn settlement_env(records: &[(Cid, FlowRecord)], change: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("at".to_string(), unix_seconds_millis()),
        ("change".to_string(), change.to_string()),
        ("dseq".to_string(), next_dseq(records).to_string()),
    ])
}

fn joined_closures(closures: &BTreeMap<String, String>) -> String {
    closures
        .iter()
        .map(|(project, closure)| format!("{project}={closure}"))
        .collect::<Vec<_>>()
        .join(",")
}

pub(crate) fn parse_joined_closures(raw: &str) -> BTreeMap<String, String> {
    raw.split(',')
        .filter_map(|pair| pair.split_once('='))
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

fn slot<'a>(slots: &'a [String], prefix: &str) -> Option<&'a str> {
    let mut values = slots.iter().filter_map(|value| value.strip_prefix(prefix));
    let value = values.next()?;
    values.next().is_none().then_some(value)
}

enum PairRule {
    HexValue,
    Capsule,
}

fn parse_pairs(
    values: &[String],
    flag: &str,
    rule: PairRule,
) -> FlowResult<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    for raw in values {
        if raw.contains(['\n', '\r']) {
            return Err(FlowError::InvalidArguments(format!(
                "{flag} values must be one line"
            )));
        }
        let (key, value) = raw
            .split_once('=')
            .ok_or_else(|| FlowError::InvalidArguments(format!("{flag} needs k=v, got `{raw}`")))?;
        if key.is_empty() || value.is_empty() {
            return Err(FlowError::InvalidArguments(format!(
                "{flag} needs nonempty k=v"
            )));
        }
        match rule {
            PairRule::HexValue if !is_hex(value) => {
                return Err(FlowError::InvalidArguments(format!(
                    "{flag} `{raw}` needs a hexadecimal closure key"
                )))
            }
            PairRule::Capsule
                if !key.starts_with(|c: char| c.is_ascii_lowercase())
                    || !key
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') =>
            {
                return Err(FlowError::InvalidArguments(format!(
                    "{flag} key `{key}` must match [a-z][a-z0-9_]*"
                )))
            }
            _ => {}
        }
        if out.insert(key.to_string(), value.to_string()).is_some() {
            return Err(FlowError::InvalidArguments(format!(
                "{flag} key `{key}` was supplied more than once"
            )));
        }
    }
    Ok(out)
}

fn validate_change(raw: &str) -> FlowResult<()> {
    if raw.len() == 41 && raw.starts_with('I') && is_hex(&raw[1..]) {
        Ok(())
    } else {
        Err(FlowError::InvalidArguments(format!(
            "--change must match I followed by 40 lowercase hexadecimal digits, got `{raw}`"
        )))
    }
}

fn validate_oid(raw: &str, flag: &str) -> FlowResult<()> {
    if matches!(raw.len(), 40 | 64) && is_hex(raw) {
        Ok(())
    } else {
        Err(FlowError::InvalidArguments(format!(
            "{flag} needs a 40- or 64-hex oid, got `{raw}`"
        )))
    }
}

fn is_hex(raw: &str) -> bool {
    !raw.is_empty()
        && raw
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn parse_cid(raw: &str) -> FlowResult<Cid> {
    raw.parse()
        .map_err(|_| FlowError::InvalidArguments(format!("`{raw}` is not a content identifier")))
}

fn unix_seconds_millis() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    format!("{}.{:03}", millis / 1000, millis % 1000)
}

pub(crate) fn unix_slot_ms(raw: &str) -> Option<u64> {
    let (seconds, fraction) = raw.split_once('.').unwrap_or((raw, "0"));
    let seconds = seconds.parse::<u64>().ok()?;
    let mut digits = fraction.bytes().take(3).collect::<Vec<_>>();
    while digits.len() < 3 {
        digits.push(b'0');
    }
    let millis = std::str::from_utf8(&digits).ok()?.parse::<u64>().ok()?;
    seconds.checked_mul(1000)?.checked_add(millis)
}

fn store_name(root: &Path) -> String {
    std::fs::canonicalize(root)
        .unwrap_or_else(|_| root.to_path_buf())
        .display()
        .to_string()
}

#[cfg(feature = "peer")]
fn served_store_guard(root: &Path) -> FlowResult<()> {
    let dir = crate::peer::peer_dir().map_err(|error| {
        FlowError::Unavailable(format!("this device's peer directory: {error}"))
    })?;
    let table = eprfs_peer::HeadTable::load(&dir).map_err(|error| {
        FlowError::Unavailable(format!(
            "the head table in {} is unreadable or does not parse ({error}); an intent written now could sit in a store no sibling reads — restart epr peer serve, which rewrites it",
            dir.display()
        ))
    })?;
    let Some(served) = table.store else {
        return Ok(());
    };
    if !served.log.is_file() {
        return Err(FlowError::Unavailable(format!(
            "the channel store `{}` that epr peer serve recorded is gone; restart epr peer serve on this device's checkout",
            served.log.display()
        )));
    }
    let requested = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let served_root = std::fs::canonicalize(&served.root).unwrap_or(served.root);
    if requested != served_root {
        return Err(FlowError::Unavailable(format!(
            "this device's peer serves the channel store at {}; an intent written to {} is read by no sibling and by no epr peer intents — pass --root {}",
            served_root.display(),
            requested.display(),
            served_root.display()
        )));
    }
    Ok(())
}

#[cfg(not(feature = "peer"))]
fn served_store_guard(_root: &Path) -> FlowResult<()> {
    Ok(())
}
