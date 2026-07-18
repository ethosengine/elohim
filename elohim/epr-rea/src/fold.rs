//! Pure folds over event history — resource STATE is derived, never stored
//! (P1: storage as reconciliation controller, applied to economics).

use std::collections::BTreeMap;

use cid::Cid;
use elohim_epr::witness::{Magnitude, ReaVerb};

use crate::model::{Commitment, FlowEvent};

/// Derived state of one resource: per-(verb, unit) totals over its event history.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResourceState {
    pub event_count: u64,
    totals: BTreeMap<(String, String), f64>,
}

impl ResourceState {
    /// Total `Count` magnitude accumulated for `verb` in `unit` (0.0 if none).
    pub fn total(&self, verb: ReaVerb, unit: &str) -> f64 {
        self.totals
            .get(&(verb_key(&verb), unit.to_string()))
            .copied()
            .unwrap_or(0.0)
    }
}

fn verb_key(verb: &ReaVerb) -> String {
    format!("{verb:?}")
}

/// Fold the events touching `resource` into its derived state.
pub fn resource_state(resource: &Cid, events: &[FlowEvent]) -> ResourceState {
    let mut state = ResourceState::default();
    for event in events.iter().filter(|e| &e.resource == resource) {
        state.event_count += 1;
        if let Magnitude::Count { value, unit } = &event.quantity {
            *state
                .totals
                .entry((verb_key(&event.action), unit.clone()))
                .or_insert(0.0) += value;
        }
    }
    state
}

/// Plan-vs-actual for one commitment: which events discharge it, and how far along.
#[derive(Debug, Clone, PartialEq)]
pub struct FulfillmentStatus {
    pub commitment: Cid,
    pub event_count: u64,
    pub fulfilled_quantity: f64,
    pub expected_quantity: Option<f64>,
}

impl FulfillmentStatus {
    /// fulfilled/expected in the commitment's declared unit; None when unbounded.
    pub fn ratio(&self) -> Option<f64> {
        match self.expected_quantity {
            Some(expected) if expected > 0.0 => Some(self.fulfilled_quantity / expected),
            _ => None,
        }
    }
}

/// Fold `events` against one commitment (identified by `commitment_cid`).
/// Only events whose `fulfills` names the commitment count; quantity sums `Count`
/// magnitudes matching the commitment's declared unit.
pub fn fulfillment(
    commitment_cid: &Cid,
    commitment: &Commitment,
    events: &[FlowEvent],
) -> FulfillmentStatus {
    let expected = commitment
        .resource_spec
        .quantity
        .as_ref()
        .and_then(|m| match m {
            Magnitude::Count { value, unit } => Some((unit.clone(), *value)),
            _ => None,
        });

    let mut event_count = 0;
    let mut fulfilled_quantity = 0.0;
    for event in events
        .iter()
        .filter(|e| e.fulfills.contains(commitment_cid))
    {
        event_count += 1;
        if let Some((unit, _)) = &expected {
            if let Some(value) = count_in_unit(&event.quantity, unit) {
                fulfilled_quantity += value;
            }
        }
    }

    FulfillmentStatus {
        commitment: *commitment_cid,
        event_count,
        fulfilled_quantity,
        expected_quantity: expected.map(|(_, value)| value),
    }
}

/// Helper: the `Count` value of a magnitude when its unit matches, else None.
pub(crate) fn count_in_unit(magnitude: &Magnitude, unit: &str) -> Option<f64> {
    match magnitude {
        Magnitude::Count { value, unit: u } if u == unit => Some(*value),
        _ => None,
    }
}
