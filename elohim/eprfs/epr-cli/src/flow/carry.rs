//! `epr flow intend --carry` — a device that takes one miss of a landing intent says so with a
//! bounded carry claim, and every device reads the claims from its own fold.
//!
//! Station F4-ask, slice 2 (plan `2026-10-29-native-dev-tooling-over-peers-plan.md` §8, the five
//! tests of §9). The first slice posts a landing intent's misses: the gate projects and closure
//! keys the lander holds no verdict for. This module is the claim a carrier makes on one of them.
//!
//! **Two records.** The promise is a native `Commitment{Produce, Active, satisfies:[intent],
//! valid_until, bound}` in this device's own flow store. What crosses is one `Cite` event on the
//! intent atom, built by the landing module's one event builder like every other `dev-landing@1`
//! event, whose env slots name the project, the closure key, the misses event, the commitment's
//! CID and the claim's own `valid_until` (unix seconds). The fold appends only events, so the
//! commitment stays on its carrier and siblings read the claim from the event.
//!
//! **Identity and bound.** A carry is identified by (carrier device, intent, project, closure). A
//! replay is the same claim: it appends nothing and never moves `valid_until`. A claim written
//! before the previous one lapsed is read as a replay too, so a carrier cannot extend its own
//! hold by writing again; renewal is a new claim made after the old one lapsed.
//!
//! **What a carry is not.** It never passes a gate (only a `gate-verdict@1` does), it never
//! fences the lander, and it reserves nothing. A lander that sees a live carry from a fresh
//! sibling waits at most until the earliest of the carry's own `valid_until`, the lander's own
//! stop-waiting time and a monotonic local deadline its caller supplies; when anything is in
//! doubt (an unparseable time, a clock skew, a silent device) the answer is to run here.
//!
//! **Settlement, by fold only, by the misses rule.** A carry stops being live when its
//! `valid_until` passes or when the miss it answers is no longer open in `misses::open_misses`: a
//! `gate-verdict@1` for its project and closure (green or red) folded from this device or an
//! admitted one, or its intent landed, withdrawn, dismissed or superseded by the device that
//! announced it. A carry is written only on an open miss.
//!
//! **Two devices that claim at once** each see only their own claim until the exchange carries
//! the other's, so both may run the gate. That is harmless: the verdicts are the same closure's.
//! After the exchange the claim with the lowest commitment CID keeps it and the other carrier
//! reads `AlreadyCarried`.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, SecondsFormat, Utc};
use cid::Cid;
use elohim_epr_rea::{
    AgentRef, Bound, Commitment, CommitmentState, FlowEvent, FlowRecord, FlowStore, ReaVerb,
    ResourceSpec, SidecarFlowStore,
};
use serde::Serialize;

use super::intend::{self, EventCommon, INTENT_TAG, LANDING_MEASURE, VIA_PREFIX};
use super::measures::{self, MeasureRef, Registry};
use super::misses;
use super::note::{
    named_identity, resolve_attribution, NoteActor, ENV_SLOT_PREFIX, MEASURE_SLOT_PREFIX,
    REASON_SLOT_PREFIX,
};
use super::{head_commit_provenance, repository_agent, repository_scope, FlowError, FlowResult};

/// The `dev-landing@1` kind of the crossing carry event (its `reason:` slot).
pub(crate) const CARRY_KIND: &str = "carry";

/// Slot 0 of a carry event, as every landing event's slot 0 is `dev-landing:<kind>`.
const CARRY_TAG: &str = "dev-landing:carry";

/// Whether a device stands admitted on this device's roster, as the caller reads it.
pub type Admits<'a> = dyn Fn(&str) -> bool + 'a;

/// What the caller asked to claim, already split out of argv.
pub struct CarryRequest<'a> {
    /// The landing intent's atom CID (the resource of every `dev-landing@1` event about it).
    pub intent: &'a str,
    /// The gate project the miss names.
    pub project: &'a str,
    /// The closure key of that gate, lowercase hexadecimal.
    pub closure: &'a str,
    /// The CID of the `misses` event that posted this miss.
    pub miss: &'a str,
    /// How long the claim holds from now, seconds. Never zero.
    pub valid_for_s: u64,
    /// The landing change, when no record in this store names it for the intent.
    pub change: Option<&'a str>,
    /// The carrier's runtime envelope bound, when its caller can read one.
    pub bound: Option<Bound>,
    pub measures: Option<&'a Path>,
    pub actor: &'a NoteActor,
    /// This device's did (`None` when it stands on no roster) and the admission it reads.
    pub me: Option<&'a str>,
    pub admits: &'a dyn Fn(&str) -> bool,
}

/// The machine-facing result of one carry write (`--json` consumers read this).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CarryReceipt {
    pub intent: String,
    pub project: String,
    pub closure: String,
    pub miss: String,
    pub commitment: String,
    pub event: String,
    pub valid_until: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dseq: Option<u64>,
    pub store: String,
    /// `false` when this device already held a live claim on the same miss: a replay.
    pub appended: bool,
}

impl CarryReceipt {
    pub fn render(&self) {
        println!(
            "intend  carry     {}  {}={}  until {}{}",
            short(&self.intent),
            self.project,
            short(&self.closure),
            self.valid_until,
            if self.appended { "" } else { "  (replay)" }
        );
        println!("        commitment: {}", short(&self.commitment));
        println!("        event: {}", short(&self.event));
    }
}

/// One carry that holds now: not lapsed by its own `valid_until`, not settled by the fold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveCarry {
    pub intent: String,
    pub project: String,
    pub closure: String,
    /// The carrier's did, or `None` for this device.
    pub carrier: Option<String>,
    /// The crossing carry event's CID.
    pub event: String,
    pub commitment: String,
    pub miss: String,
    /// Unix seconds, the carrier's own clock.
    pub valid_until: u64,
    /// When the carrier wrote it, unix milliseconds on its own clock.
    pub at_ms: Option<u64>,
}

/// Which question the decision answers.
pub enum CarryRole<'a> {
    /// A device that would take the miss: run it, or another device already carries it.
    Carrier,
    /// The device whose landing has the miss: run it here, or wait a bounded time for a carry.
    Lander(LanderBounds<'a>),
}

/// The bounds a lander's wait is held to. Every value is the caller's own reading.
pub struct LanderBounds<'a> {
    /// How young a carrier's last folded head must be to count it as present, from policy.
    /// `None` when it could not be read: the lander runs here.
    pub freshness_s: Option<u64>,
    /// The clock budget between devices, from policy. `None` when it could not be read.
    pub max_skew_s: Option<u64>,
    /// When this device last folded a head from a device, unix milliseconds on this clock.
    pub last_seen_ms: &'a dyn Fn(&str) -> Option<u64>,
    /// The lander's own stop-waiting time from its misses event, unix seconds.
    pub wait_until: Option<u64>,
    /// What remains of the lander's monotonic local deadline, seconds. A wall-clock jump never
    /// moves it, so it caps every wait this decision returns.
    pub max_wait_s: u64,
}

/// One decision request over the records of this device's fold.
pub struct CarryQuery<'a> {
    pub intent: &'a Cid,
    pub project: &'a str,
    pub closure: &'a str,
    pub admits: &'a dyn Fn(&str) -> bool,
    pub me: Option<&'a str>,
    pub now_unix_s: u64,
    pub role: CarryRole<'a>,
}

/// The answer to "should this device run the gate for this miss?"
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "decision", rename_all = "kebab-case")]
pub enum CarryDecision {
    /// Carrier role: no other admitted device holds a live carry that keeps the miss.
    Run,
    /// Carrier role: another admitted device's live carry keeps the miss.
    #[serde(rename_all = "camelCase")]
    AlreadyCarried {
        by: String,
        until: u64,
        commitment: String,
    },
    /// Lander role: no carry is worth waiting for; run the gate here (`NotCarried`).
    /// Wire: `{"decision":"run-here","reason":"…"}`.
    #[serde(rename_all = "camelCase")]
    RunHere { reason: String },
    /// Lander role: a fresh sibling carries the miss; wait at most `wait_s`, then run here.
    /// Wire: `{"decision":"wait","until":<unix s>,"by":"<did>","reason":"…"}`; `wait_s` and the
    /// keeping commitment are for this process only.
    #[serde(rename_all = "camelCase")]
    Wait {
        until: u64,
        by: String,
        reason: String,
        #[serde(skip)]
        wait_s: u64,
        #[serde(skip)]
        commitment: String,
    },
}

impl CarryDecision {
    pub fn render(&self) {
        match self {
            Self::Run => println!("carry   run"),
            Self::AlreadyCarried {
                by,
                until,
                commitment,
            } => println!(
                "carry   already carried by {}  until {until}  ({})",
                short(by),
                short(commitment)
            ),
            Self::RunHere { reason } => println!("carry   not carried: run here ({reason})"),
            Self::Wait {
                by, until, wait_s, ..
            } => println!(
                "carry   carried by {}: wait at most {wait_s} s (until {until}), then run here",
                short(by)
            ),
        }
    }
}

/// Write one carry claim on a miss: the native commitment and its crossing `Cite` event.
///
/// A live claim of this device's on the same (intent, project, closure) is returned unchanged,
/// with `appended: false`, whatever `valid_for_s` asks. A live claim of another admitted device
/// that keeps the miss refuses, naming it, before anything is appended.
///
/// # Errors
///
/// [`FlowError::InvalidArguments`] for a malformed request or an incumbent carrier;
/// [`FlowError::Unavailable`] when this root is not the store `epr peer serve` projects; store and
/// measure-registry errors as `epr flow intend` reports them.
/// The longest a single carry may claim a miss. The costliest gate measured on a device is the
/// storage gate at 3,535 s; two hours covers it from a cold slot and no more.
pub const MAX_VALID_FOR_S: u64 = 7_200;

pub fn write_carry(root: &Path, request: &CarryRequest<'_>) -> FlowResult<CarryReceipt> {
    intend::served_store_guard(root)?;
    let intent = parse_cid(request.intent, "--carry")?;
    let miss = parse_cid(request.miss, "--miss")?;
    validate_project(request.project)?;
    validate_closure(request.closure)?;
    if request.valid_for_s == 0 {
        return Err(FlowError::InvalidArguments(
            "--valid-for needs a positive number of seconds: a carry is bounded".to_string(),
        ));
    }
    if request.valid_for_s > MAX_VALID_FOR_S {
        return Err(FlowError::InvalidArguments(format!(
            "--valid-for is at most {MAX_VALID_FOR_S} seconds: a longer claim would hold other \
             carriers off a miss past any gate's measured cost; claim again when it lapses"
        )));
    }
    if let Some(change) = request.change {
        validate_one_line(change, "--change")?;
    }

    let registry_path = request
        .measures
        .map(Path::to_path_buf)
        .unwrap_or_else(|| measures::default_measures(root));
    let measure = MeasureRef::parse(LANDING_MEASURE, &registry_path)?;
    let registry = Registry::open(&registry_path)?;
    if !registry.declares(&measure) {
        return Err(FlowError::InvalidArguments(format!(
            "unknown measure `{LANDING_MEASURE}` — declare it in {} before writing a carry",
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
    let attribution = resolve_attribution(root, named, request.actor.session.as_deref(), &author);
    let provider = attribution.provider(&author);
    let receiver = repository_agent(root)?;
    let scope = repository_scope(root)?;

    let mut store = SidecarFlowStore::open(root)?.transaction()?;
    let records = store.records()?;
    let now_ms = unix_ms();
    let now_s = now_ms / 1000;

    // A replay first: this device's own live claim on the same miss is the answer, unchanged.
    let live = live_carries(&records, request.admits, request.me, now_s);
    if let Some(own) = live.iter().find(|carry| {
        carry.carrier.is_none()
            && carry.intent == intent.to_string()
            && carry.project == request.project
            && carry.closure == request.closure
    }) {
        return Ok(receipt_from_live(root, own, &records, false));
    }
    if let CarryDecision::AlreadyCarried {
        by,
        until,
        commitment,
    } = decide(
        &records,
        &CarryQuery {
            intent: &intent,
            project: request.project,
            closure: request.closure,
            admits: request.admits,
            me: request.me,
            now_unix_s: now_s,
            role: CarryRole::Carrier,
        },
    ) {
        return Err(FlowError::InvalidArguments(format!(
            "{}={} of intent {intent} is already carried by {by} until {until} (commitment {commitment}); a second carry would only run it twice",
            request.project, request.closure
        )));
    }
    require_open_miss(&records, request, &intent, &miss)?;
    let change = match request.change {
        Some(change) => change.to_string(),
        None => intent_change(&records, &intent).ok_or_else(|| {
            FlowError::InvalidArguments(format!(
                "no record in this store names the change of intent {intent}; fold its misses event first or pass --change"
            ))
        })?,
    };

    let valid_until = now_s.checked_add(request.valid_for_s).ok_or_else(|| {
        FlowError::InvalidArguments("--valid-for overflows the clock".to_string())
    })?;
    let commitment = Commitment {
        action: ReaVerb::Produce,
        provider: AgentRef(provider.clone()),
        receiver: receiver.clone(),
        resource_spec: ResourceSpec {
            quantity: None,
            classified_as: commitment_slots(&change, request, &miss, &attribution),
        },
        in_scope_of: scope,
        valid_from: Some(rfc3339(now_s)),
        valid_until: Some(rfc3339(valid_until)),
        state: CommitmentState::Active,
        satisfies: vec![intent],
        bound: request.bound.clone(),
    };
    let commitment = FlowRecord::Commitment(commitment);
    let commitment_cid = commitment.cid()?;

    let dseq = intend::next_dseq(&records);
    let at = intend::unix_seconds_millis();
    let env = BTreeMap::from([
        ("at".to_string(), at.clone()),
        ("change".to_string(), change),
        ("closure".to_string(), request.closure.to_string()),
        ("commitment".to_string(), commitment_cid.to_string()),
        ("dseq".to_string(), dseq.to_string()),
        ("miss".to_string(), miss.to_string()),
        ("project".to_string(), request.project.to_string()),
        ("valid_until".to_string(), valid_until.to_string()),
    ]);
    let common = EventCommon {
        provider: &provider,
        receiver: &receiver,
        scope: &scope,
        occurred_at: &occurred_at,
        unit_slot: unit_slot.as_deref(),
        attribution: &attribution,
    };
    let event = intend::event_record(CARRY_KIND, ReaVerb::Cite, intent, vec![], env, &common);
    let event_cid = event.cid()?;

    if !records.iter().any(|(cid, _)| *cid == commitment_cid) {
        store.append(commitment)?;
    }
    store.append(event)?;
    Ok(CarryReceipt {
        intent: intent.to_string(),
        project: request.project.to_string(),
        closure: request.closure.to_string(),
        miss: miss.to_string(),
        commitment: commitment_cid.to_string(),
        event: event_cid.to_string(),
        valid_until,
        at: Some(at),
        dseq: Some(dseq),
        store: intend::store_name(root),
        appended: true,
    })
}

/// The records of the flow store at `root`, or none when it has no store yet.
///
/// # Errors
///
/// The store's own read errors.
pub fn fold_records(root: &Path) -> FlowResult<Vec<(Cid, FlowRecord)>> {
    if root.join(".eprfs/status/flows.jsonl").is_file() {
        Ok(SidecarFlowStore::open(root)?.records()?)
    } else {
        Ok(Vec::new())
    }
}

/// Every carry that holds at `now_unix_s` in this device's fold, one per (carrier, intent,
/// project, closure), in a stable order.
///
/// A carry counts only from this device (no `via:` slot) or from a device `admits` admits; a
/// record folded back from this device itself (`via:` = `me`) is not counted twice. A claim
/// written while the same carrier's previous claim on the same miss still held is a replay and
/// never extends it. A lapsed carry, or one settled by the fold (see the module documentation),
/// is excluded.
#[must_use]
pub fn live_carries(
    records: &[(Cid, FlowRecord)],
    admits: &dyn Fn(&str) -> bool,
    me: Option<&str>,
    now_unix_s: u64,
) -> Vec<LiveCarry> {
    let mut chains: BTreeMap<(Option<String>, String, String, String), Vec<Claim>> =
        BTreeMap::new();
    for (cid, record) in records {
        let FlowRecord::Event(event) = record else {
            continue;
        };
        let Some(claim) = claim_from_event(*cid, event, admits, me) else {
            continue;
        };
        chains
            .entry((
                claim.carrier.clone(),
                claim.intent.to_string(),
                claim.project.clone(),
                claim.closure.clone(),
            ))
            .or_default()
            .push(claim);
    }
    let mut live = Vec::new();
    for (_, mut claims) in chains {
        // The carrier's own order: its record counter, which a clock step cannot reorder.
        claims.sort_by(|left, right| {
            (left.dseq, left.at_ms, left.event.to_bytes()).cmp(&(
                right.dseq,
                right.at_ms,
                right.event.to_bytes(),
            ))
        });
        let mut current: Option<Claim> = None;
        for claim in claims {
            let renews = match &current {
                None => true,
                Some(held) => claim.at_ms.is_some_and(|at| at / 1000 >= held.valid_until),
            };
            if renews {
                current = Some(claim);
            }
        }
        let Some(claim) = current else {
            continue;
        };
        if now_unix_s >= claim.valid_until
            || miss_settled(
                records,
                admits,
                me,
                &claim.intent.to_string(),
                &claim.project,
                &claim.closure,
            )
        {
            continue;
        }
        live.push(LiveCarry {
            intent: claim.intent.to_string(),
            project: claim.project,
            closure: claim.closure,
            carrier: claim.carrier,
            event: claim.event.to_string(),
            commitment: claim.commitment.to_string(),
            miss: claim.miss,
            valid_until: claim.valid_until,
            at_ms: claim.at_ms,
        });
    }
    live
}

/// Answer one carrier's or one lander's question about one miss, from this device's fold.
#[must_use]
pub fn decide(records: &[(Cid, FlowRecord)], query: &CarryQuery<'_>) -> CarryDecision {
    let intent = query.intent.to_string();
    let matching: Vec<LiveCarry> = live_carries(records, query.admits, query.me, query.now_unix_s)
        .into_iter()
        .filter(|carry| {
            carry.intent == intent
                && carry.project == query.project
                && carry.closure == query.closure
        })
        .collect();
    match &query.role {
        CarryRole::Carrier => carrier_decision(&matching),
        CarryRole::Lander(bounds) => lander_decision(&matching, bounds, query.now_unix_s),
    }
}

fn carrier_decision(matching: &[LiveCarry]) -> CarryDecision {
    let Some(keeper) = lowest_commitment(matching.iter().filter(|c| c.carrier.is_some())) else {
        return CarryDecision::Run;
    };
    let own = lowest_commitment(matching.iter().filter(|c| c.carrier.is_none()));
    if own.is_some_and(|own| commitment_key(own) < commitment_key(keeper)) {
        return CarryDecision::Run;
    }
    CarryDecision::AlreadyCarried {
        by: keeper.carrier.clone().unwrap_or_default(),
        until: keeper.valid_until,
        commitment: keeper.commitment.clone(),
    }
}

fn lander_decision(
    matching: &[LiveCarry],
    bounds: &LanderBounds<'_>,
    now_unix_s: u64,
) -> CarryDecision {
    let run_here = |reason: &str| CarryDecision::RunHere {
        reason: reason.to_string(),
    };
    let siblings: Vec<&LiveCarry> = matching.iter().filter(|c| c.carrier.is_some()).collect();
    if siblings.is_empty() {
        return run_here("no admitted device carries this miss");
    }
    let (Some(freshness_s), Some(max_skew_s)) = (bounds.freshness_s, bounds.max_skew_s) else {
        return run_here(
            "berth.channel.freshness_s or max_skew_s is unreadable from the pool policy; no guess is made",
        );
    };
    if bounds.wait_until.is_some_and(|until| until <= now_unix_s) {
        return run_here("this landing's own stop-waiting time has passed");
    }
    if bounds.max_wait_s == 0 {
        return run_here("this landing's local deadline has passed");
    }
    let now_ms = now_unix_s.saturating_mul(1000);
    let fresh = siblings.into_iter().filter(|carry| {
        let carrier = carry.carrier.as_deref().unwrap_or_default();
        // A head folded in this device's future means its clock stepped back: in doubt.
        let present = (bounds.last_seen_ms)(carrier)
            .is_some_and(|seen| seen <= now_ms && now_ms - seen < freshness_s.saturating_mul(1000));
        // A carry written further ahead of this clock than the skew budget is in doubt too.
        let sane = carry
            .at_ms
            .is_some_and(|at| at <= now_ms.saturating_add(max_skew_s.saturating_mul(1000)));
        present && sane
    });
    let Some(keeper) = lowest_commitment(fresh) else {
        return run_here("no carrier of this miss has a fresh head, or its clock is in doubt");
    };
    let mut until = keeper.valid_until;
    if let Some(own) = bounds.wait_until {
        until = until.min(own);
    }
    until = until.min(now_unix_s.saturating_add(bounds.max_wait_s));
    let wait_s = until.saturating_sub(now_unix_s);
    if wait_s == 0 {
        return run_here("the wait bound has passed");
    }
    let by = keeper.carrier.clone().unwrap_or_default();
    CarryDecision::Wait {
        reason: format!(
            "{by} carries this miss under commitment {}; waiting at most {wait_s} s, then running here",
            keeper.commitment
        ),
        by,
        until,
        wait_s,
        commitment: keeper.commitment.clone(),
    }
}

/// The deterministic tie-break both devices compute alike: the lowest commitment CID's bytes.
fn lowest_commitment<'a>(carries: impl Iterator<Item = &'a LiveCarry>) -> Option<&'a LiveCarry> {
    carries.min_by(|left, right| commitment_key(left).cmp(&commitment_key(right)))
}

fn commitment_key(carry: &LiveCarry) -> Vec<u8> {
    carry.commitment.parse::<Cid>().map_or_else(
        |_| carry.commitment.clone().into_bytes(),
        |cid| cid.to_bytes(),
    )
}

/// The open miss this carry answers: the one `open_misses` folds for (intent, project, closure),
/// and its posting event must be the misses event the carrier named.
fn require_open_miss(
    records: &[(Cid, FlowRecord)],
    request: &CarryRequest<'_>,
    intent: &Cid,
    miss: &Cid,
) -> FlowResult<()> {
    let intent_text = intent.to_string();
    let open = misses::open_misses(records, request.admits, request.me);
    let Some(found) = open.iter().find(|open| {
        open.intent == intent_text
            && open.project == request.project
            && open.closure == request.closure
    }) else {
        return Err(FlowError::InvalidArguments(format!(
            "no open miss {}={} on intent {intent} in this device's fold: it was never posted here, or a verdict or the intent's settlement already closed it",
            request.project, request.closure
        )));
    };
    if found.event != miss.to_string() {
        return Err(FlowError::InvalidArguments(format!(
            "the open miss {}={} on intent {intent} was posted by event {}, not {miss}",
            request.project, request.closure, found.event
        )));
    }
    Ok(())
}

/// The lander's own stop-waiting time for one of its misses, as its misses event carries it.
#[must_use]
pub fn own_wait_until(
    records: &[(Cid, FlowRecord)],
    admits: &dyn Fn(&str) -> bool,
    me: Option<&str>,
    intent: &Cid,
    project: &str,
    closure: &str,
) -> Option<u64> {
    let intent = intent.to_string();
    misses::open_misses(records, admits, me)
        .into_iter()
        .find(|open| {
            open.announcer.is_none()
                && open.intent == intent
                && open.project == project
                && open.closure == closure
        })?
        .wait_until
}

struct Claim {
    event: Cid,
    carrier: Option<String>,
    intent: Cid,
    project: String,
    closure: String,
    miss: String,
    commitment: Cid,
    valid_until: u64,
    at_ms: Option<u64>,
    dseq: Option<u64>,
}

/// One crossing carry event read as a claim, or `None` when it is not one, names a field twice,
/// or comes from a device that is not admitted.
fn claim_from_event(
    cid: Cid,
    event: &FlowEvent,
    admits: &dyn Fn(&str) -> bool,
    me: Option<&str>,
) -> Option<Claim> {
    let slots = &event.classified_as;
    if slots.first().map(String::as_str) != Some(CARRY_TAG)
        || intend::slot(slots, MEASURE_SLOT_PREFIX) != Some(LANDING_MEASURE)
        || intend::slot(slots, REASON_SLOT_PREFIX) != Some(CARRY_KIND)
    {
        return None;
    }
    let carrier = match writer(event)? {
        Some(device) if Some(device) == me || !admits(device) => return None,
        other => other.map(str::to_string),
    };
    let env = |key: &str| intend::slot(slots, &format!("{ENV_SLOT_PREFIX}{key}="));
    Some(Claim {
        event: cid,
        carrier,
        intent: event.resource,
        project: env("project")?.to_string(),
        closure: env("closure")?.to_string(),
        miss: env("miss")?.to_string(),
        commitment: env("commitment")?.parse().ok()?,
        valid_until: env("valid_until")?.parse().ok()?,
        at_ms: env("at").and_then(intend::unix_slot_ms),
        dseq: env("dseq").and_then(|raw| raw.parse().ok()),
    })
}

/// The device that wrote a folded event (`Some(Some(did))`), this device (`Some(None)`), or
/// `None` when the event names more than one writer and so names none.
fn writer(event: &FlowEvent) -> Option<Option<&str>> {
    let mut vias = event
        .classified_as
        .iter()
        .filter_map(|value| value.strip_prefix(VIA_PREFIX));
    let first = vias.next();
    vias.next().is_none().then_some(first)
}

/// Whether the miss `(intent, project, closure)` is settled, by exactly the rule `open_misses`
/// folds, so a carry and a miss settle by one rule: a `gate-verdict@1` for that project and
/// closure (green or red) from this or an admitted device, or the intent landed, withdrawn,
/// dismissed or superseded by its announcer. A miss no event ever posted is settled too, since
/// there is nothing open to carry.
///
/// The rule has one home, `misses::miss_settled`, which also settles an older intent's misses
/// when its announcer posts newer misses or a landing for the same change.
fn miss_settled(
    records: &[(Cid, FlowRecord)],
    admits: &dyn Fn(&str) -> bool,
    me: Option<&str>,
    intent: &str,
    project: &str,
    closure: &str,
) -> bool {
    misses::miss_settled(records, admits, me, intent, project, closure)
}

/// The change an intent is about, from its own atom or any landing event that names it.
fn intent_change(records: &[(Cid, FlowRecord)], intent: &Cid) -> Option<String> {
    records.iter().find_map(|(cid, record)| match record {
        FlowRecord::Intent(atom)
            if cid == intent
                && atom.resource_spec.classified_as.first().map(String::as_str)
                    == Some(INTENT_TAG) =>
        {
            atom.resource_spec.classified_as.get(1).cloned()
        }
        FlowRecord::Event(event)
            if event.resource == *intent
                && intend::slot(&event.classified_as, MEASURE_SLOT_PREFIX)
                    == Some(LANDING_MEASURE) =>
        {
            intend::slot(&event.classified_as, &format!("{ENV_SLOT_PREFIX}change="))
                .filter(|change| !change.is_empty())
                .map(str::to_string)
        }
        _ => None,
    })
}

fn commitment_slots(
    change: &str,
    request: &CarryRequest<'_>,
    miss: &Cid,
    attribution: &super::note::Attribution,
) -> Vec<String> {
    let mut slots = vec![
        CARRY_TAG.to_string(),
        change.to_string(),
        format!("project:{}", request.project),
        format!("closure:{}={}", request.project, request.closure),
        format!("miss:{miss}"),
    ];
    attribution.append_slots(&mut slots);
    slots
}

fn receipt_from_live(
    root: &Path,
    carry: &LiveCarry,
    records: &[(Cid, FlowRecord)],
    appended: bool,
) -> CarryReceipt {
    let event = records.iter().find_map(|(cid, record)| match record {
        FlowRecord::Event(event) if cid.to_string() == carry.event => Some(event),
        _ => None,
    });
    let env = |key: &str| {
        event.and_then(|event| {
            intend::slot(&event.classified_as, &format!("{ENV_SLOT_PREFIX}{key}="))
        })
    };
    CarryReceipt {
        intent: carry.intent.clone(),
        project: carry.project.clone(),
        closure: carry.closure.clone(),
        miss: carry.miss.clone(),
        commitment: carry.commitment.clone(),
        event: carry.event.clone(),
        valid_until: carry.valid_until,
        at: env("at").map(str::to_string),
        dseq: env("dseq").and_then(|raw| raw.parse().ok()),
        store: intend::store_name(root),
        appended,
    }
}

fn parse_cid(raw: &str, flag: &str) -> FlowResult<Cid> {
    raw.parse().map_err(|_| {
        FlowError::InvalidArguments(format!("{flag} `{raw}` is not a content identifier"))
    })
}

fn validate_project(raw: &str) -> FlowResult<()> {
    let ok = !raw.is_empty()
        && raw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if ok {
        Ok(())
    } else {
        Err(FlowError::InvalidArguments(format!(
            "--project needs a gate project name ([A-Za-z0-9._-]+), got `{raw}`"
        )))
    }
}

fn validate_closure(raw: &str) -> FlowResult<()> {
    if intend::is_hex(raw) {
        Ok(())
    } else {
        Err(FlowError::InvalidArguments(format!(
            "--closure needs a lowercase hexadecimal closure key, got `{raw}`"
        )))
    }
}

fn validate_one_line(raw: &str, flag: &str) -> FlowResult<()> {
    if raw.is_empty() || raw.contains(['\n', '\r', ',']) {
        Err(FlowError::InvalidArguments(format!(
            "{flag} must be one nonempty line"
        )))
    } else {
        Ok(())
    }
}

fn rfc3339(unix_s: u64) -> String {
    i64::try_from(unix_s)
        .ok()
        .and_then(|secs| DateTime::<Utc>::from_timestamp(secs, 0))
        .map_or_else(
            || unix_s.to_string(),
            |at| at.to_rfc3339_opts(SecondsFormat::Secs, true),
        )
}

/// This device's clock, unix milliseconds.
fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        })
}

fn short(raw: &str) -> String {
    raw.parse::<Cid>().map_or_else(
        |_| {
            if raw.len() > 16 {
                format!("{}…", &raw[..16])
            } else {
                raw.to_string()
            }
        },
        |cid| super::short_cid(&cid),
    )
}

#[cfg(feature = "peer")]
pub(crate) use device::DeviceFold;

/// This device as its fold knows it: its did, its admission, the heads it folded and the policy
/// budgets. Read for the CLI only; the decisions above stay pure over records.
#[cfg(feature = "peer")]
mod device {
    use std::path::{Path, PathBuf};

    use eprfs_peer::{Admission, HeadTable};

    use crate::actor::{channel_standing, ChannelStanding};
    use crate::device_key::{self, DeviceKey};
    use crate::peer::{peer_dir, Device, POLICY_REL};

    pub(crate) struct DeviceFold {
        pub(crate) root: PathBuf,
        pub(crate) me: Option<String>,
        admission: Option<Admission>,
        heads: Option<HeadTable>,
        pub(crate) freshness_s: Option<u64>,
        pub(crate) max_skew_s: Option<u64>,
    }

    impl DeviceFold {
        /// Read what can be read; anything unreadable is left `None`, which the decisions read as
        /// doubt (a lander runs here, a carrier sees no sibling).
        pub(crate) fn read(root: &Path) -> Self {
            let heads = peer_dir().ok().and_then(|dir| HeadTable::load(&dir).ok());
            let store_root = heads
                .as_ref()
                .and_then(|table| table.store.as_ref())
                .map_or_else(|| root.to_path_buf(), |place| place.root.clone());
            let policy = heads
                .as_ref()
                .and_then(|table| table.rules.as_ref())
                .map_or_else(|| root.join(POLICY_REL), |rules| rules.policy.clone());
            let (freshness_s, max_skew_s) = channel_budgets(&policy);
            let (me, admission) = standing(&store_root);
            Self {
                root: store_root,
                me,
                admission,
                heads,
                freshness_s,
                max_skew_s,
            }
        }

        pub(crate) fn admits(&self, device: &str) -> bool {
            self.admission
                .as_ref()
                .is_some_and(|admission| admission.admits_device(device))
        }

        /// When this device last folded a head from `device`, unix ms on this clock.
        pub(crate) fn last_seen_ms(&self, device: &str) -> Option<u64> {
            self.heads
                .as_ref()?
                .devices
                .get(device)
                .map(|head| head.folded_at)
        }
    }

    fn standing(root: &Path) -> (Option<String>, Option<Admission>) {
        let Some(key_file) = device_key::resolve_path()
            .ok()
            .filter(|path| path.is_file())
        else {
            return (None, None);
        };
        let Ok(key) = DeviceKey::load(&key_file) else {
            return (None, None);
        };
        let did = key.did_key();
        let Ok(ChannelStanding::Stands { handle }) = channel_standing(root, None, &did) else {
            return (None, None);
        };
        let admission = Device::open(root, Some(&handle))
            .and_then(|device| device.admission())
            .ok()
            .filter(|admission| admission.admits_device(&did));
        match admission {
            Some(admission) => (Some(did), Some(admission)),
            None => (None, None),
        }
    }

    /// `berth.channel.freshness_s` and `berth.channel.max_skew_s` from the pool policy.
    fn channel_budgets(policy: &Path) -> (Option<u64>, Option<u64>) {
        let Some(channel) = std::fs::read(policy)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .and_then(|value| value.get("berth")?.get("channel").cloned())
        else {
            return (None, None);
        };
        (
            channel
                .get("freshness_s")
                .and_then(serde_json::Value::as_u64),
            channel
                .get("max_skew_s")
                .and_then(serde_json::Value::as_u64),
        )
    }
}

#[cfg(test)]
mod tests;
