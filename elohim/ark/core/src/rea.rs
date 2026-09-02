//! Projections of the runtime's own records into the substrate's REA/ValueFlows vocabulary.
//!
//! The ark mints **no new economic vocabulary**. A lifecycle action is already expressible with
//! the six [`ReaVerb`]s the substrate speaks, plus the established two-slot `classified_as`
//! convention — **tag first, subject second**: `["runtime:<action>", "<process name>"]`. That is
//! why there is no new verb, no new EPR kind, and no new record type here: only pure functions
//! from ark-core's records onto [`elohim_epr_rea::model`] atoms.
//!
//! Nothing in this module writes. The supervisor crate owns the sidecar store and is its only
//! construction site; ark-core takes `elohim-epr-rea` with `default-features = false` so the
//! store is not even compiled into the pure half (`boundary::no_fs_in_pure_core`).

use std::str::FromStr;

use chrono::{DateTime, SecondsFormat, Utc};
use cid::Cid;
use elohim_epr::measure::{ClaimKind, Confidence, Interval, MeasureKind, Period, Quantity};
use elohim_epr_rea::model::{
    atom_cid, AgentRef, Bound, FlowEvent, Intent as ReaIntent, PinnedRef, Process as ReaProcess,
    ResourceSpec,
};
use elohim_epr_rea::stock::Window;
use elohim_epr_rea::{AlgedonicEvidence, Magnitude, ReaVerb};

use crate::{
    berth::Berth,
    intent::{Intent, IntentAction},
    manifest::ChildPolicy,
    passport::Passport,
    sample::ProcessSample,
    tally::DeathTally,
    verdict::{BoundedBy, GiveUpReason, RestartGrant},
    witness::{DeathWitness, Incident, IncidentClose},
};

/// `classified_as` tag for a first spawn of a child in an incarnation.
pub const RUNTIME_TAG_SPAWN: &str = "runtime:spawn";
/// `classified_as` tag for a policy-driven restart after a death.
pub const RUNTIME_TAG_RESTART: &str = "runtime:restart";
/// `classified_as` tag for a graceful stop request.
pub const RUNTIME_TAG_STOP: &str = "runtime:stop";
/// `classified_as` tag for a forced termination.
pub const RUNTIME_TAG_KILL: &str = "runtime:kill";
/// `classified_as` tag for a permanent give-up on a child.
pub const RUNTIME_TAG_GIVE_UP: &str = "runtime:give-up";
/// `classified_as` tag for an observed child death.
pub const RUNTIME_TAG_DEATH: &str = "runtime:death";
/// `classified_as` tag for the terminal outcome of an incident.
pub const RUNTIME_TAG_INCIDENT_CLOSED: &str = "runtime:incident-closed";

/// Unit of the quantity a death event carries: how much process-time was consumed.
pub const UNIT_PROCESS_MS: &str = "process-ms";
/// Unit an intensity bound and an incident's death count are denominated in.
pub const UNIT_DEATHS: &str = "deaths";

/// Recipe identity every runtime incident is a run of.
///
/// An incident is a real VF [`ReaProcess`] — deaths in, a terminal outcome out — but S0 mints
/// no recipe atom for it, so the pin names the vocabulary rather than a stored
/// [`elohim_epr_rea::model::ProcessSpec`]. S1 replaces the id with that spec's own id; the
/// version is the declared-dependency slot, never "whatever is latest".
pub const INCIDENT_SPEC_ID: &str = "runtime:incident";
/// Declared version of [`INCIDENT_SPEC_ID`] this crate projects against.
pub const INCIDENT_SPEC_VERSION: u32 = 1;

/// Stand-in agent for a berth that has not yet been bound to an identity.
///
/// [`AgentRef`] is non-optional on every VF atom, and S0 berths routinely carry no node CID.
/// An empty string would be indistinguishable from a bug; this sentinel is legible as exactly
/// what it is, and a reader can filter on it.
pub const UNIDENTIFIED_AGENT: &str = "runtime:unidentified";

/// Failure while projecting an ark record into the substrate vocabulary.
#[derive(thiserror::Error, Clone, Debug, PartialEq, Eq)]
pub enum ReaProjectionError {
    /// A stored identity string is not a CID.
    #[error("not a CID: {0}")]
    Cid(String),
    /// Canonical encoding of a projected atom failed.
    #[error("projection encoding: {0}")]
    Encode(String),
    /// A manifest policy does not describe a well-formed bound.
    #[error("invalid bound: {0}")]
    Bound(String),
    /// A recorded millisecond timestamp is not a representable instant.
    #[error("not a representable timestamp: {0} ms")]
    Timestamp(u64),
}

/// The scope every runtime projection is accountable to.
///
/// Built **once** per berth by the supervisor and threaded into the sink, so every projected
/// atom names the same container: `scope` is the runtime manifest's CID (VF `in_scope_of`),
/// `node` the berth's agent when identity is bound, and `bounded_by` the self-contract whose
/// promise the runtime's flows discharge — `None` under plain manifest policy, which is why
/// [`GiveUpReason::as_algedonic`] can be honestly absent rather than reporting a zero.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeScope {
    /// CID of the runtime manifest occupying the berth.
    pub scope: Cid,
    /// Agent bound to the berth, once identity is available.
    pub node: Option<AgentRef>,
    /// Commitment bounding this runtime's flows, once one is minted (S1).
    pub bounded_by: Option<Cid>,
}

impl RuntimeScope {
    /// A scope naming only its manifest — no identity, no bounding commitment.
    pub fn new(scope: Cid) -> Self {
        Self {
            scope,
            node: None,
            bounded_by: None,
        }
    }

    /// Binds the berth's agent identity.
    pub fn with_node(mut self, node: AgentRef) -> Self {
        self.node = Some(node);
        self
    }

    /// Binds the commitment whose promise this runtime's flows discharge.
    pub fn with_bounded_by(mut self, commitment: Cid) -> Self {
        self.bounded_by = Some(commitment);
        self
    }

    /// Builds the scope a berth declares: its manifest CID, and its node when bound.
    pub fn from_berth(berth: &Berth) -> Result<Self, ReaProjectionError> {
        let mut scope = Self::new(parse_cid(&berth.manifest)?);
        if let Some(node) = &berth.node {
            scope.node = Some(AgentRef(node.clone()));
        }
        Ok(scope)
    }

    /// The agent every projected atom is attributed to, or [`UNIDENTIFIED_AGENT`].
    pub fn agent(&self) -> AgentRef {
        self.node
            .clone()
            .unwrap_or_else(|| AgentRef(UNIDENTIFIED_AGENT.to_string()))
    }
}

impl IntentAction {
    /// The substrate verb this lifecycle action already is.
    ///
    /// Starting a child **produces** the running process; stopping or killing it **consumes**
    /// it; giving up **dismisses** the claim that it should run at all. No seventh verb.
    pub fn rea_verb(&self) -> ReaVerb {
        match self {
            Self::Spawn | Self::Restart { .. } => ReaVerb::Produce,
            Self::Stop { .. } | Self::Kill => ReaVerb::Consume,
            Self::GiveUp => ReaVerb::Dismiss,
        }
    }

    /// The `classified_as` tag naming this action within the `runtime:` vocabulary.
    pub fn runtime_tag(&self) -> &'static str {
        match self {
            Self::Spawn => RUNTIME_TAG_SPAWN,
            Self::Restart { .. } => RUNTIME_TAG_RESTART,
            Self::Stop { .. } => RUNTIME_TAG_STOP,
            Self::Kill => RUNTIME_TAG_KILL,
            Self::GiveUp => RUNTIME_TAG_GIVE_UP,
        }
    }
}

impl Intent {
    /// Projects this write-ahead decision as a VF [`ReaIntent`] — a desired flow, in scope.
    ///
    /// The quantity is deliberately absent: an intent to start or stop a child promises no
    /// measured amount. What was actually consumed is the death event's business
    /// ([`DeathWitness::as_flow_event`]).
    pub fn as_rea_intent(&self, scope: &RuntimeScope) -> ReaIntent {
        ReaIntent {
            action: self.action.rea_verb(),
            resource_spec: ResourceSpec {
                classified_as: vec![self.action.runtime_tag().to_string(), self.process.clone()],
                quantity: None,
            },
            in_scope_of: scope.scope,
            raised_by: scope.agent(),
        }
    }
}

impl DeathWitness {
    /// Projects this death as the VF [`FlowEvent`] it already is.
    ///
    /// The resource flowed is the witness itself — content-addressed, so it IS a resource — and
    /// the quantity is the process-time the incarnation consumed before dying. `fulfills` names
    /// the runtime's self-contract when one exists, which is the same edge the DHT spells
    /// `bounded_by`.
    pub fn as_flow_event(&self, scope: &RuntimeScope) -> Result<FlowEvent, ReaProjectionError> {
        let resource = elohim_epr::cid::compute_cid(
            &self
                .canonical_bytes()
                .map_err(|error| ReaProjectionError::Encode(error.to_string()))?,
        );
        Ok(FlowEvent {
            action: ReaVerb::Consume,
            provider: scope.agent(),
            receiver: scope.agent(),
            resource,
            quantity: Magnitude::Count {
                value: self.uptime_ms as f64,
                unit: UNIT_PROCESS_MS.to_string(),
            },
            process: None,
            in_scope_of: scope.scope,
            fulfills: scope.bounded_by.into_iter().collect(),
            satisfies: Vec::new(),
            occurred_at: rfc3339_ms(self.died_at_epoch_ms)?,
            classified_as: vec![RUNTIME_TAG_DEATH.to_string(), self.process.clone()],
        })
    }
}

impl IncidentClose {
    /// The `close:` slot naming which terminal outcome this is.
    ///
    /// The third `classified_as` slot, prefixed so a reader can tell it from the leading tag.
    pub fn close_slot(&self) -> &'static str {
        match self {
            Self::ReadyAgain { .. } => "close:ready-again",
            Self::GaveUp { .. } => "close:gave-up",
            Self::Stopped { .. } => "close:stopped",
        }
    }

    /// Wall-clock time of the terminal outcome.
    pub fn at_epoch_ms(&self) -> u64 {
        match self {
            Self::ReadyAgain { at_epoch_ms }
            | Self::GaveUp { at_epoch_ms, .. }
            | Self::Stopped { at_epoch_ms } => *at_epoch_ms,
        }
    }
}

impl Incident {
    /// The output event carrying this incident's terminal outcome, once it has one.
    ///
    /// An open incident has produced no outcome, so this is `None` — honest absence, not an
    /// event asserting a close that has not happened.
    pub fn as_close_event(
        &self,
        scope: &RuntimeScope,
    ) -> Result<Option<FlowEvent>, ReaProjectionError> {
        let Some(close) = &self.closed else {
            return Ok(None);
        };
        Ok(Some(FlowEvent {
            action: ReaVerb::Produce,
            provider: scope.agent(),
            receiver: scope.agent(),
            resource: parse_cid(&self.id)?,
            quantity: Magnitude::Count {
                value: self.witnesses.len() as f64,
                unit: UNIT_DEATHS.to_string(),
            },
            process: None,
            in_scope_of: scope.scope,
            fulfills: scope.bounded_by.into_iter().collect(),
            satisfies: Vec::new(),
            occurred_at: rfc3339_ms(close.at_epoch_ms())?,
            classified_as: vec![
                RUNTIME_TAG_INCIDENT_CLOSED.to_string(),
                self.process.clone(),
                close.close_slot().to_string(),
            ],
        }))
    }

    /// Projects this incident as the VF [`ReaProcess`] it already is: the deaths it holds are
    /// its inputs, and its terminal outcome is its one output.
    pub fn as_rea_process(&self, scope: &RuntimeScope) -> Result<ReaProcess, ReaProjectionError> {
        let inputs = self
            .witnesses
            .iter()
            .map(|witness| parse_cid(witness))
            .collect::<Result<Vec<_>, _>>()?;
        let outputs =
            match self.as_close_event(scope)? {
                Some(event) => vec![atom_cid(&event)
                    .map_err(|error| ReaProjectionError::Encode(error.to_string()))?],
                None => Vec::new(),
            };
        Ok(ReaProcess {
            spec: PinnedRef {
                id: INCIDENT_SPEC_ID.to_string(),
                version: INCIDENT_SPEC_VERSION,
            },
            in_scope_of: scope.scope,
            inputs,
            outputs,
        })
    }
}

impl ChildPolicy {
    /// The intensity limit expressed as the substrate's [`Bound`] — a ceiling this runtime
    /// declares on its own promise, denominated in [`UNIT_DEATHS`].
    ///
    /// `threshold_pct` is a **percentage** in `[1, 100]` (`85.0` means 85%), the band edge at
    /// which an approach fires before the limit itself; [`Bound::new`] refuses anything else.
    /// Ceiling is the v1 sense and encodes as an absent `sense`, so this bound has exactly one
    /// spelling and therefore exactly one CID.
    pub fn intensity_bound(&self, threshold_pct: f64) -> Result<Bound, ReaProjectionError> {
        Bound::new(
            f64::from(self.intensity.max_deaths),
            UNIT_DEATHS.to_string(),
            threshold_pct,
        )
        .map_err(|error| ReaProjectionError::Bound(error.to_string()))
    }
}

impl DeathTally {
    /// Projects the tally's sliding window as the substrate's [`Window`].
    ///
    /// [`DeathTally::deaths_within`] counts an **inclusive** `[now - window_s, now]`; `Window`
    /// is half-open `[start, end)`. The end is therefore the second *after* `now`, so the two
    /// agree on every death — pinned by `the_window_admits_exactly_the_deaths_the_tally_counts`.
    pub fn as_window(&self, now_epoch_s: u64, window_s: u64) -> Result<Window, ReaProjectionError> {
        Ok(Window {
            start: rfc3339_s(now_epoch_s.saturating_sub(window_s))?,
            end: rfc3339_s(now_epoch_s.saturating_add(1))?,
            per: Period::Second,
            periods: (window_s + 1) as f64,
        })
    }
}

impl ProcessSample {
    /// Projects the present measurements as named [`Quantity`] levels — **at the boundary
    /// only**.
    ///
    /// The raw struct stays on the wire inside a witness: a `Quantity` costs a
    /// [`Confidence`] per field, and manufacturing one per sample would triple a witness's
    /// bytes to say nothing a reader could not derive. A consumer that needs dimensional
    /// safety (comparing an RSS level against a limit, dividing a CPU time by a window) calls
    /// this and gets kinds it cannot silently mis-divide.
    ///
    /// Absent measurements are absent, never zero — a missing `/proc` read is not a process
    /// that used nothing.
    pub fn as_quantities(&self) -> Vec<(&'static str, Quantity)> {
        let mut quantities = Vec::new();
        let mut push = |name: &'static str, value: f64, unit: &str| {
            quantities.push((
                name,
                Quantity {
                    value,
                    kind: MeasureKind::Level,
                    confidence: Confidence {
                        // The kernel measured it; no agent witnessed or estimated it.
                        claim: ClaimKind::InstrumentMeasured,
                        interval: Interval::exact(value),
                        basis: format!("ark process sample, {unit}, from /proc and rusage"),
                        unknown_reason: None,
                    },
                },
            ));
        };

        if let Some(v) = self.max_rss_bytes {
            push("max-rss-bytes", v as f64, "bytes");
        }
        if let Some(v) = self.rss_bytes {
            push("rss-bytes", v as f64, "bytes");
        }
        if let Some(v) = self.user_us {
            push("user-us", v as f64, "microseconds");
        }
        if let Some(v) = self.system_us {
            push("system-us", v as f64, "microseconds");
        }
        if let Some(v) = self.fds {
            push("fds", f64::from(v), "descriptors");
        }
        if let Some(v) = self.threads {
            push("threads", f64::from(v), "threads");
        }
        if let Some(v) = self.io_read_bytes {
            push("io-read-bytes", v as f64, "bytes");
        }
        if let Some(v) = self.io_write_bytes {
            push("io-write-bytes", v as f64, "bytes");
        }
        if let Some(v) = self.oom_score_adj {
            push("oom-score-adj", f64::from(v), "score");
        }
        quantities
    }
}

impl BoundedBy {
    /// The bounding commitment's CID, when authority comes from one.
    ///
    /// [`Self::ManifestPolicy`] is `None` on purpose: a manifest default is the operator's
    /// line, not a promise anyone made, so there is nothing for pain to be evidence *against*.
    pub fn commitment_cid(&self) -> Option<&str> {
        match self {
            Self::ManifestPolicy => None,
            Self::Commitment { cid } => Some(cid),
        }
    }
}

impl GiveUpReason {
    /// The algedonic breach this give-up is evidence of, against the named bounding
    /// commitment.
    ///
    /// [`Self::PolicyTemporary`] is `None`: a child declared never to restart did not breach
    /// anything by not restarting.
    ///
    /// The limit is read off the refusal itself, which each gate reaches at a known crossing:
    /// the same-cause gate refuses on `run >= limit`, so at the moment of refusal the run *is*
    /// the limit; the intensity gate refuses on the first death *past* the ceiling, so the
    /// ceiling is one below the observed count. When the grant carries its declared
    /// [`Bound`], prefer [`RestartGrant::pain`], which reads the limit instead of inferring
    /// it.
    pub fn as_algedonic(&self, bound_ref: &str) -> Option<AlgedonicEvidence> {
        match self {
            Self::PolicyTemporary => None,
            Self::SameCause { count, .. } => Some(AlgedonicEvidence::Breach {
                stock: f64::from(*count),
                limit: f64::from(*count),
                bound_ref: bound_ref.to_string(),
            }),
            Self::IntensityExceeded { deaths, .. } => Some(AlgedonicEvidence::Breach {
                stock: f64::from(*deaths),
                limit: f64::from(deaths.saturating_sub(1)),
                bound_ref: bound_ref.to_string(),
            }),
        }
    }
}

impl RestartGrant {
    /// The pain this give-up reports — `Some` only under [`BoundedBy::Commitment`].
    ///
    /// Under manifest policy there is no promise to be in pain about, so this is `None`:
    /// honest absence, never a zero. When the grant carries its declared [`Bound`], that
    /// limit replaces the one inferred from the refusal.
    pub fn pain(&self, reason: &GiveUpReason) -> Option<AlgedonicEvidence> {
        let bound_ref = self.bounded_by.commitment_cid()?;
        let evidence = reason.as_algedonic(bound_ref)?;
        let Some(bound) = &self.bound else {
            return Some(evidence);
        };
        Some(AlgedonicEvidence::Breach {
            stock: evidence.stock(),
            limit: bound.limit,
            bound_ref: bound_ref.to_string(),
        })
    }
}

impl Passport {
    /// The passport is a **live projection**, not a flow: it says what is true now, and REA
    /// atoms say what was intended, promised, or observed. Projecting it as an event would
    /// mint one atom per refresh addressing the same fact.
    ///
    /// It is therefore KEPT as-is and carried inside a witness. What a reader wants from it —
    /// how much process-time the incarnation consumed — is already the death event's quantity.
    pub fn deaths_in_window(&self, process: &str) -> Option<u32> {
        self.processes
            .iter()
            .find(|p| p.name == process)
            .map(|p| p.deaths_in_window)
    }
}

fn parse_cid(s: &str) -> Result<Cid, ReaProjectionError> {
    Cid::from_str(s).map_err(|_| ReaProjectionError::Cid(s.to_string()))
}

/// RFC3339 in UTC, second precision — the uniform format `Window`'s lexicographic comparison
/// requires.
fn rfc3339_s(epoch_s: u64) -> Result<String, ReaProjectionError> {
    let seconds =
        i64::try_from(epoch_s).map_err(|_| ReaProjectionError::Timestamp(epoch_s * 1_000))?;
    DateTime::<Utc>::from_timestamp(seconds, 0)
        .map(|t| t.to_rfc3339_opts(SecondsFormat::Secs, true))
        .ok_or(ReaProjectionError::Timestamp(epoch_s.saturating_mul(1_000)))
}

/// RFC3339 in UTC, millisecond precision — same uniform shape, finer grain for events.
fn rfc3339_ms(epoch_ms: u64) -> Result<String, ReaProjectionError> {
    let millis = i64::try_from(epoch_ms).map_err(|_| ReaProjectionError::Timestamp(epoch_ms))?;
    DateTime::<Utc>::from_timestamp_millis(millis)
        .map(|t| t.to_rfc3339_opts(SecondsFormat::Millis, true))
        .ok_or(ReaProjectionError::Timestamp(epoch_ms))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        exit::ExitClass,
        manifest::Intensity,
        passport::{EffectiveTier, ProcessPassport, PASSPORT_KIND},
        tally::DeathRecord,
        witness::WITNESS_KIND,
        RestartVerdict,
    };

    fn cid_of(label: &str) -> Cid {
        elohim_epr::cid::compute_cid(label.as_bytes())
    }

    fn scope() -> RuntimeScope {
        RuntimeScope::new(cid_of("manifest")).with_node(AgentRef("uhCAk-matthew".to_string()))
    }

    fn bounded_scope() -> RuntimeScope {
        scope().with_bounded_by(cid_of("self-contract"))
    }

    fn passport() -> Passport {
        Passport {
            schema: 1,
            kind: PASSPORT_KIND.to_string(),
            manifest: "bafy-manifest".to_string(),
            node: None,
            incarnation: 3,
            ark_version: "0.1.0".to_string(),
            processes: vec![ProcessPassport {
                name: "conductor".to_string(),
                artifact_sha256: "ab".repeat(32),
                artifact_path: "/opt/elohim/conductor".to_string(),
                pid: Some(42),
                started_at_epoch_ms: Some(1_000),
                ready: false,
                effective_tier: EffectiveTier::None,
                deaths_in_window: 2,
            }],
            last_verdict: Some(RestartVerdict::Stop),
            updated_at_epoch_ms: 2_000,
        }
    }

    fn witness() -> DeathWitness {
        DeathWitness {
            schema: 1,
            kind: WITNESS_KIND.to_string(),
            incident: cid_of("incident").to_string(),
            process: "conductor".to_string(),
            incarnation: 3,
            pid: 42,
            artifact_sha256: "ab".repeat(32),
            artifact_path: "/opt/elohim/conductor".to_string(),
            started_at_epoch_ms: 1_000,
            died_at_epoch_ms: 4_500,
            uptime_ms: 3_500,
            exit: ExitClass::Signaled {
                signal: 9,
                core_dumped: false,
            },
            last_stderr: Vec::new(),
            last_stdout: Vec::new(),
            sample: None,
            last_intent: None,
            passport: passport(),
            verdict: None,
            refusal: None,
            bounded_by: None,
            pain: None,
        }
    }

    fn death(at_epoch_s: u64) -> DeathRecord {
        DeathRecord {
            at_epoch_s,
            class: ExitClass::Exited { code: 1 },
            uptime_ms: 1_000,
            first_stderr_line: None,
        }
    }

    #[test]
    fn an_intent_projects_onto_an_existing_verb_with_the_runtime_tag_and_scope() {
        let scope = scope();
        let cases = [
            (IntentAction::Spawn, ReaVerb::Produce, RUNTIME_TAG_SPAWN),
            (
                IntentAction::Restart {
                    attempt: 1,
                    after_s: 2,
                },
                ReaVerb::Produce,
                RUNTIME_TAG_RESTART,
            ),
            (
                IntentAction::Stop {
                    signal: 2,
                    grace_ms: 20_000,
                },
                ReaVerb::Consume,
                RUNTIME_TAG_STOP,
            ),
            (IntentAction::Kill, ReaVerb::Consume, RUNTIME_TAG_KILL),
            (IntentAction::GiveUp, ReaVerb::Dismiss, RUNTIME_TAG_GIVE_UP),
        ];

        for (action, verb, tag) in cases {
            let intent = Intent {
                at_epoch_ms: 1_000,
                incarnation: 3,
                process: "conductor".to_string(),
                action,
                reason: "because".to_string(),
            };

            let projected = intent.as_rea_intent(&scope);

            assert_eq!(projected.action, verb, "{tag}");
            assert_eq!(
                projected.resource_spec.classified_as,
                vec![tag.to_string(), "conductor".to_string()],
                "tag first, subject second"
            );
            assert_eq!(projected.in_scope_of, scope.scope);
            assert_eq!(projected.raised_by, AgentRef("uhCAk-matthew".to_string()));
        }
    }

    #[test]
    fn an_unbound_berth_projects_the_unidentified_agent_not_an_empty_string() {
        let scope = RuntimeScope::new(cid_of("manifest"));

        assert_eq!(scope.agent(), AgentRef(UNIDENTIFIED_AGENT.to_string()));
        assert!(!UNIDENTIFIED_AGENT.is_empty());
    }

    #[test]
    fn a_runtime_scope_is_built_from_the_berth_it_serves() {
        let berth = Berth {
            manifest: cid_of("manifest").to_string(),
            node: Some("uhCAk-matthew".to_string()),
            ..Berth::default()
        };

        let scope = RuntimeScope::from_berth(&berth).unwrap();

        assert_eq!(scope.scope, cid_of("manifest"));
        assert_eq!(scope.node, Some(AgentRef("uhCAk-matthew".to_string())));
        assert_eq!(scope.bounded_by, None);

        let unbound = Berth {
            manifest: "not-a-cid".to_string(),
            ..Berth::default()
        };
        assert_eq!(
            RuntimeScope::from_berth(&unbound),
            Err(ReaProjectionError::Cid("not-a-cid".to_string()))
        );
    }

    #[test]
    fn a_death_projects_as_a_consume_event_of_process_ms_fulfilling_the_self_contract() {
        let scope = bounded_scope();
        let witness = witness();

        let event = witness.as_flow_event(&scope).unwrap();

        assert_eq!(event.action, ReaVerb::Consume);
        assert_eq!(
            event.resource,
            elohim_epr::cid::compute_cid(&witness.canonical_bytes().unwrap()),
            "the resource flowed is the witness itself"
        );
        assert_eq!(
            event.quantity,
            Magnitude::Count {
                value: 3_500.0,
                unit: UNIT_PROCESS_MS.to_string(),
            }
        );
        assert_eq!(event.in_scope_of, scope.scope);
        assert_eq!(event.fulfills, vec![cid_of("self-contract")]);
        assert_eq!(
            event.classified_as,
            vec![RUNTIME_TAG_DEATH.to_string(), "conductor".to_string()]
        );
        assert_eq!(event.occurred_at, "1970-01-01T00:00:04.500Z");
    }

    #[test]
    fn an_unbounded_runtime_projects_a_death_with_no_fulfills_edge() {
        let event = witness().as_flow_event(&scope()).unwrap();

        assert!(
            event.fulfills.is_empty(),
            "no self-contract means no fulfillment edge to invent"
        );
    }

    #[test]
    fn an_incident_projects_as_a_process_whose_inputs_are_its_deaths() {
        let scope = bounded_scope();
        let mut incident = Incident::open("conductor", 1_000, 3);
        incident.witnesses = vec![cid_of("w1").to_string(), cid_of("w2").to_string()];

        let projected = incident.as_rea_process(&scope).unwrap();

        assert_eq!(
            projected.spec,
            PinnedRef {
                id: INCIDENT_SPEC_ID.to_string(),
                version: INCIDENT_SPEC_VERSION,
            }
        );
        assert_eq!(projected.in_scope_of, scope.scope);
        assert_eq!(projected.inputs, vec![cid_of("w1"), cid_of("w2")]);
        assert!(
            projected.outputs.is_empty(),
            "an open incident has produced no outcome"
        );
        assert_eq!(incident.as_close_event(&scope).unwrap(), None);
    }

    #[test]
    fn a_closed_incident_produces_one_tagged_output_event() {
        let scope = bounded_scope();
        let mut incident = Incident::open("conductor", 1_000, 3);
        incident.witnesses = vec![cid_of("w1").to_string()];
        incident.closed = Some(IncidentClose::GaveUp {
            at_epoch_ms: 9_000,
            reason: GiveUpReason::PolicyTemporary,
        });

        let event = incident.as_close_event(&scope).unwrap().unwrap();

        assert_eq!(event.action, ReaVerb::Produce);
        assert_eq!(event.resource, parse_cid(&incident.id).unwrap());
        assert_eq!(
            event.quantity,
            Magnitude::Count {
                value: 1.0,
                unit: UNIT_DEATHS.to_string(),
            }
        );
        assert_eq!(
            event.classified_as,
            vec![
                RUNTIME_TAG_INCIDENT_CLOSED.to_string(),
                "conductor".to_string(),
                "close:gave-up".to_string(),
            ]
        );
        assert_eq!(event.in_scope_of, scope.scope);
        assert_eq!(event.fulfills, vec![cid_of("self-contract")]);
        assert_eq!(event.occurred_at, "1970-01-01T00:00:09.000Z");

        let projected = incident.as_rea_process(&scope).unwrap();
        assert_eq!(projected.outputs, vec![atom_cid(&event).unwrap()]);
    }

    #[test]
    fn the_intensity_policy_is_a_ceiling_bound_denominated_in_deaths() {
        let policy = ChildPolicy {
            intensity: Intensity {
                max_deaths: 5,
                window_s: 300,
            },
            ..ChildPolicy::default()
        };

        let bound = policy.intensity_bound(80.0).unwrap();

        assert_eq!(bound.limit, 5.0);
        assert_eq!(bound.unit, UNIT_DEATHS);
        assert_eq!(bound.threshold_pct, 80.0);
        assert_eq!(
            bound.sense, None,
            "ceiling is the v1 default and has exactly one encoding"
        );
        assert!(bound.breached_by(5.0));
        assert!(bound.approached_by(4.0));

        // A fraction where a percentage belongs is the documented confusion the substrate
        // refuses, and the refusal reaches this projection unchanged.
        assert!(matches!(
            policy.intensity_bound(0.85),
            Err(ReaProjectionError::Bound(_))
        ));
    }

    #[test]
    fn the_window_admits_exactly_the_deaths_the_tally_counts() {
        let mut tally = DeathTally::default();
        for at in [0, 100, 200, 300, 400, 600] {
            tally.record(death(at));
        }

        let window = tally.as_window(600, 300).unwrap();

        assert_eq!(window.per, Period::Second);
        assert_eq!(window.periods, 301.0);
        assert_eq!(tally.deaths_within(600, 300), 3);
        let admitted = tally
            .deaths
            .iter()
            .filter(|d| window.contains(&rfc3339_s(d.at_epoch_s).unwrap()))
            .count();
        assert_eq!(
            admitted, 3,
            "the half-open Window and the inclusive tally must agree on every death"
        );
    }

    #[test]
    fn a_sample_projects_only_the_measurements_it_actually_has() {
        let sparse = ProcessSample {
            max_rss_bytes: Some(2_048),
            ..ProcessSample::default()
        };

        let quantities = sparse.as_quantities();

        assert_eq!(quantities.len(), 1, "absent is absent, never zero");
        let (name, quantity) = &quantities[0];
        assert_eq!(*name, "max-rss-bytes");
        assert_eq!(quantity.value, 2_048.0);
        assert_eq!(quantity.kind, MeasureKind::Level);
        assert_eq!(quantity.confidence.claim, ClaimKind::InstrumentMeasured);
        assert!(quantity.confidence.basis.contains("bytes"));

        assert!(ProcessSample::default().as_quantities().is_empty());
    }

    #[test]
    fn pain_is_absent_under_manifest_policy_and_present_under_a_commitment() {
        let reason = GiveUpReason::IntensityExceeded {
            deaths: 6,
            window_s: 300,
        };
        let policy = ChildPolicy::default();

        let manifest_grant = RestartGrant {
            bounded_by: BoundedBy::ManifestPolicy,
            policy: policy.clone(),
            bound: None,
        };
        assert_eq!(
            manifest_grant.pain(&reason),
            None,
            "a manifest default is nobody's promise, so there is nothing to be in pain about"
        );

        let commitment_grant = RestartGrant {
            bounded_by: BoundedBy::Commitment {
                cid: "bafy-commitment".to_string(),
            },
            policy: policy.clone(),
            bound: None,
        };
        assert_eq!(
            commitment_grant.pain(&reason),
            Some(AlgedonicEvidence::Breach {
                stock: 6.0,
                limit: 5.0,
                bound_ref: "bafy-commitment".to_string(),
            })
        );

        // A grant carrying its declared bound reports the declared limit, not an inferred one.
        let declared = RestartGrant {
            bounded_by: BoundedBy::Commitment {
                cid: "bafy-commitment".to_string(),
            },
            policy: ChildPolicy {
                intensity: Intensity {
                    max_deaths: 4,
                    window_s: 300,
                },
                ..policy
            },
            bound: Some(Bound::new(4.0, UNIT_DEATHS.to_string(), 80.0).unwrap()),
        };
        assert_eq!(
            declared.pain(&reason),
            Some(AlgedonicEvidence::Breach {
                stock: 6.0,
                limit: 4.0,
                bound_ref: "bafy-commitment".to_string(),
            })
        );
    }

    #[test]
    fn a_temporary_policy_give_up_is_no_breach_at_all() {
        assert_eq!(
            GiveUpReason::PolicyTemporary.as_algedonic("bafy-commitment"),
            None
        );
        assert_eq!(
            GiveUpReason::SameCause {
                key: "signaled:9||fast:true".to_string(),
                count: 3,
            }
            .as_algedonic("bafy-commitment"),
            Some(AlgedonicEvidence::Breach {
                stock: 3.0,
                limit: 3.0,
                bound_ref: "bafy-commitment".to_string(),
            })
        );
        assert_eq!(BoundedBy::ManifestPolicy.commitment_cid(), None);
        assert_eq!(
            BoundedBy::Commitment {
                cid: "bafy-commitment".to_string()
            }
            .commitment_cid(),
            Some("bafy-commitment")
        );
    }
}
