//! D.8 — the reconcile loop and its ports.
//!
//! Event-driven: no polling timer. Two clock-born triggers remain, both bounded: `PeriodClosed`
//! (one per period per peer; it settles and never re-places) and `CarrierLost` (at most once per
//! carrier per freshness horizon, emitted by the adapter, D4).

use cid::Cid;
use elohim_epr_rea::AgentRef;

use std::collections::BTreeSet;

use crate::adopt::{
    fulfilment_id, settle, Carrying, CommitmentDraft, FulfilmentDraft, Period, RealEvidence,
    Release,
};
use crate::decide::{decide, Snapshot};
use crate::kind::{KindId, KindRegistry, MethodRegistry};
use crate::offer::EpochSecs;
use crate::rank::Scratch;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trigger {
    PostureChanged(AgentRef),
    MembershipChanged,
    NeedExpressed(Cid),
    NeedWithdrawn(Cid),
    Miss(Cid),
    CommitmentObserved(String),
    ParametersDeclared {
        scope: Cid,
        kind: KindId,
        set: Cid,
    },
    /// Clock-born (D4): emitted by the custody `Ports` adapter when a carrier's inventory or presence
    /// crosses the charter's freshness horizon, at most once per carrier per horizon.
    CarrierLost(AgentRef),
    /// Clock-born: one per period per peer; it settles and never re-places.
    PeriodClosed(Period),
}

/// A skip test only (D16): a trigger whose affected set is empty skips the pass; every pass that
/// runs is whole, so capacity is charged the same way as a full pass.
pub fn affected(trigger: &Trigger, s: &Snapshot) -> bool {
    let carrier_is = |a: &AgentRef| {
        s.candidates.iter().any(|c| c.agent() == a)
            || s.carrying.iter().any(|c| s.normalize(&c.carrier) == a)
    };
    match trigger {
        Trigger::PostureChanged(a) => carrier_is(a),
        Trigger::MembershipChanged => true,
        Trigger::NeedExpressed(id) => s.needs.iter().any(|n| n.id().ok().as_ref() == Some(id)),
        Trigger::NeedWithdrawn(id) => s.carrying.iter().any(|c| c.need == *id),
        Trigger::Miss(cid) => s
            .needs
            .iter()
            .any(|n| n.key.cid() == Some(cid) || n.id().ok().as_ref() == Some(cid)),
        Trigger::CommitmentObserved(id) => s.carrying.iter().any(|c| c.commitment == *id),
        Trigger::ParametersDeclared { scope, kind, .. } => s
            .needs
            .iter()
            .any(|n| n.in_scope_of == *scope && n.kind == *kind),
        Trigger::CarrierLost(a) => s.carrying.iter().any(|c| s.normalize(&c.carrier) == a),
        Trigger::PeriodClosed(_) => false,
    }
}

#[derive(Debug, thiserror::Error)]
#[error("port: {0}")]
pub struct PortError(pub String);

/// Sync, like `salvage_pass`; async adapters bridge with `block_in_place` exactly as
/// `SalvageCommitmentAuthor` does today. Implemented outside this crate.
pub trait Ports {
    /// One read pass into a reused snapshot. One grouped query per table, never one per provider per
    /// blob. It calls `select_in_force` (§E) for each (scope, kind).
    fn snapshot(
        &self,
        scopes: &[Cid],
        now: EpochSecs,
        into: &mut Snapshot,
    ) -> Result<(), PortError>;
    /// Idempotent by id.
    fn author(&self, draft: &CommitmentDraft) -> Result<(), PortError>;
    fn release(&self, release: &Release) -> Result<(), PortError>;
    /// Idempotent by `FulfilmentDraft::id` (one per commitment per period, `fulfilment_id`): a
    /// second draft with an id already recorded records nothing, so a re-delivered `PeriodClosed`
    /// never settles twice.
    fn settle(&self, f: &FulfilmentDraft) -> Result<(), PortError>;
    /// What a carrying of mine held over a closed period, with the real readings that show it,
    /// built with `RealEvidence::from_readings(carrying, period, ..)` from real, present readings of
    /// the kind's fulfilment promise observed in the period; evidence for another commitment or
    /// period, or a quantity above what the readings support, is refused by `settle` and settles
    /// nothing. `None`: nothing to settle (no reading, or none that qualifies). Defaulted, and additive to the spec: `PeriodClosed` settles, and
    /// settling needs `RealEvidence` (I9), which no snapshot field holds.
    fn evidence(&self, _carrying: &Carrying, _period: Period) -> Option<(u64, RealEvidence)> {
        None
    }
}

pub struct Floor<P: Ports> {
    ports: P,
    kinds: KindRegistry,
    methods: MethodRegistry,
    scopes: Vec<Cid>,
    snapshot: Snapshot,
    scratch: Scratch,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct PassOutcome {
    pub adopted: u32,
    pub released: u32,
    pub settled: u32,
    pub unserved: u32,
    pub observed_only: u32,
}

impl<P: Ports> Floor<P> {
    /// `me` is filled by the first `Ports::snapshot`.
    pub fn new(ports: P, kinds: KindRegistry, methods: MethodRegistry, scopes: Vec<Cid>) -> Self {
        Self {
            ports,
            kinds,
            methods,
            scopes,
            snapshot: Snapshot::empty(AgentRef(String::new())),
            scratch: Scratch::default(),
        }
    }

    pub fn ports(&self) -> &P {
        &self.ports
    }

    /// Event-driven: no polling timer. The adapter coalesces triggers under the posture hysteresis
    /// into one call.
    pub fn on(&mut self, trigger: &Trigger, now: EpochSecs) -> Result<PassOutcome, PortError> {
        self.ports.snapshot(&self.scopes, now, &mut self.snapshot)?;
        let mut outcome = PassOutcome::default();
        if let Trigger::PeriodClosed(period) = trigger {
            let me = self.snapshot.me.clone();
            // One settlement per commitment per period within this call too (a commitment observed
            // under two scopes is one commitment); across calls, `Ports::settle` is idempotent by id.
            let mut drafted: BTreeSet<String> = BTreeSet::new();
            for c in &self.snapshot.carrying {
                if self.snapshot.normalize(&c.carrier) != &me
                    || c.honored != seam_contracts::Answer::Present(true)
                    || drafted.contains(&fulfilment_id(&c.commitment, *period))
                {
                    continue;
                }
                let Some((quantity, evidence)) = self.ports.evidence(c, *period) else {
                    continue;
                };
                // Evidence for another commitment or period, or an unsupported quantity, settles
                // nothing.
                let Ok(mut draft) = settle(c, *period, quantity, evidence) else {
                    continue;
                };
                draft.provider = me.clone();
                drafted.insert(draft.id.clone());
                self.ports.settle(&draft)?;
                outcome.settled += 1;
            }
            return Ok(outcome);
        }
        if !affected(trigger, &self.snapshot) {
            return Ok(outcome);
        }
        let plan = decide(
            &self.snapshot,
            &self.kinds,
            &self.methods,
            &mut self.scratch,
        );
        for draft in &plan.adopt {
            self.ports.author(draft)?;
            outcome.adopted += 1;
        }
        for release in &plan.release {
            self.ports.release(release)?;
            outcome.released += 1;
        }
        for f in &plan.settle {
            self.ports.settle(f)?;
            outcome.settled += 1;
        }
        outcome.unserved = plan.unserved.len() as u32;
        outcome.observed_only = plan.observe_only.len() as u32;
        Ok(outcome)
    }
}
